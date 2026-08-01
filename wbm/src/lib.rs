//! Wayback Machine (WTJ) archive snapshot integration for Truth Social types.
//!
//! This crate plugs [`truthsocial::model::Status`] into the `archivindex-wbm-json` snapshot
//! infrastructure. [`validation_context`] builds the Truth Social [`Context`] (with the gzip
//! codec), [`WtjSnapshot`] wraps a snapshot's [`StatusContent`], [`read_statuses`] streams statuses
//! out of a zstd-compressed compact snapshot file, and [`merge_compact`] merges two such files in
//! digest order. The [`cdx`] module reads directories of CDX entry files.
#![warn(clippy::all, clippy::pedantic, clippy::nursery, rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use archivindex_wbm::digest::Sha1Digest;
use archivindex_wbm_json::context::Context;
use archivindex_wbm_json::exact::ExactSnapshot;
use bounded_static::IntoBoundedStatic;
use either::Either;
use truthsocial::model::{Status, StatusContent};

pub mod cdx;

/// A WTJ snapshot whose content is the deserialized [`StatusContent`]: a single [`Status`] or, for
/// endpoints that return a list (e.g. paged video feeds), an array of them.
pub type WtjSnapshot<'a> = archivindex_wbm_json::Snapshot<'a, StatusContent<'a>>;

/// A WTJ snapshot whose content is the exact serialized bytes, so it can be parsed,
/// digest-validated, and re-emitted byte-for-byte. This is the form used when reading and merging
/// compact files.
pub type CompactSnapshot<'a> = ExactSnapshot<'a>;

/// The bundled Truth Social context configuration (default closing whitespace and the status-URL
/// CEL query). Deserialized into an [`archivindex_wbm_json::context::ContextConfig`].
const CONTEXT_CONFIG: &str = include_str!("truthsocial.toml");

/// Builds a [`Context`] for reading, validating, and serializing compact WTJ snapshot files.
///
/// This is the library's canonical Truth Social context, defined by the bundled
/// [`truthsocial.toml`](CONTEXT_CONFIG) (default closing whitespace `['\n']` and the status-URL CEL
/// inference) and extended with the [`gzip`](archivindex_wbm_json_gzip) codec. A snapshot whose
/// content is decompressed from a gzip archive carries the reproduction parameters in its `format`
/// object, so it validates against the digest of its original compressed bytes.
///
/// # Panics
///
/// Panics if the bundled configuration is not valid (invalid TOML or an invalid CEL `url_query`,
/// both of which are checked while deserializing [`ContextConfig`]), which would be a build-time
/// bug rather than a runtime condition.
///
/// [`ContextConfig`]: archivindex_wbm_json::context::ContextConfig
#[must_use]
pub fn validation_context() -> Context {
    let config: archivindex_wbm_json::context::ContextConfig = toml::from_str(CONTEXT_CONFIG)
        .expect("bundled truthsocial.toml should be valid TOML with a valid url_query");
    let mut context = Context::from_config(config);
    archivindex_wbm_json_gzip::register(&mut context);
    context
}

/// An error encountered while reading a compact WTJ snapshot file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be opened, decompressed, or read.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A line could not be parsed as a [`WtjSnapshot`].
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Streams Truth Social statuses from a zstd-compressed compact WTJ snapshot file.
///
/// The file is zstd-compressed with one JSON [`WtjSnapshot`] per line. Each yielded item is one
/// owned [`Status`]; a snapshot whose content is an array of statuses yields each of them in turn.
/// Blank lines are skipped. Statuses are produced lazily, so the whole file is never held in memory
/// at once.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be opened or its zstd header is invalid. Each iterator
/// item is an [`Error::Io`] for a read failure or an [`Error::Json`] for a line that fails to
/// deserialize.
pub fn read_statuses<P: AsRef<Path>>(
    path: P,
) -> Result<impl Iterator<Item = Result<Status<'static>, Error>>, Error> {
    let decoder = zstd::Decoder::new(File::open(path)?)?;

    Ok(BufReader::new(decoder)
        .lines()
        .filter_map(|line| match line {
            Err(error) => Some(Err(Error::Io(error))),
            Ok(line) if line.trim().is_empty() => None,
            // The snapshot envelope's optional string fields borrow from the input, so the envelope
            // is parsed at `line`'s lifetime; its content is deserialized as owned, which is what
            // lets the yielded statuses outlive the line they came from.
            Ok(line) => Some(
                serde_json::from_str::<archivindex_wbm_json::Snapshot<'_, StatusContent<'static>>>(
                    &line,
                )
                .map(|snapshot| snapshot.content)
                .map_err(Error::Json),
            ),
        })
        // Flatten each snapshot's content into its individual statuses (one for a single status,
        // several for an array), threading any parse error through as a single item.
        .flat_map(|result| match result {
            Ok(content) => Either::Left(content.into_statuses().map(Ok)),
            Err(error) => Either::Right(std::iter::once(Err(error))),
        }))
}

/// An error encountered while merging two compact WTJ snapshot files.
#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    /// An input file could not be opened, decompressed, or read.
    #[error("failed to read {}", .path.display())]
    Read {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A line of an input file could not be parsed as a compact snapshot.
    #[error("failed to parse snapshot on line {line} of {}", .path.display())]
    Parse {
        /// The file containing the bad line.
        path: PathBuf,
        /// The 1-based line number that failed to parse.
        line: usize,
        /// The underlying parse error.
        #[source]
        source: archivindex_wbm_json::Error,
    },
    /// An input file is not sorted by digest, which streaming merge requires.
    #[error("{} is not sorted by digest (line {line})", .path.display())]
    NotSorted {
        /// The file that is out of order.
        path: PathBuf,
        /// The 1-based line number whose digest is smaller than the preceding one.
        line: usize,
    },
    /// The merged output could not be written.
    #[error("failed to write merged output")]
    Write(#[source] std::io::Error),
    /// Two rows share a digest but differ in some field (a hash collision or inconsistent data).
    #[error("digest collision: multiple differing rows share digest {digest}")]
    Collision {
        /// The digest shared by the conflicting rows.
        digest: Sha1Digest,
    },
}

/// Summary of a [`merge_compact`] run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MergeSummary {
    /// Total rows read from both input files.
    pub read: usize,
    /// Duplicate rows dropped (identical digest and contents).
    pub duplicates: usize,
    /// Distinct rows written to the output.
    pub written: usize,
}

/// Merges two compact WTJ snapshot files into `writer`, ordered by SHA-1 digest byte order.
///
/// Both inputs must already be sorted by digest. The merge streams them in lockstep, holding only
/// the current digest group in memory rather than loading either file. Rows that are exact
/// duplicates (same digest and identical in every field) are collapsed into one and logged at
/// `warn` level. Rows that share a digest but differ in any field are treated as a collision and
/// abort the merge with [`MergeError::Collision`].
///
/// Content is preserved byte-for-byte, so the digests in the output remain valid. The caller is
/// responsible for flushing or finalizing `writer`.
///
/// # Errors
///
/// [`MergeError::Read`] / [`MergeError::Parse`] for input problems, [`MergeError::NotSorted`] if an
/// input is not in ascending digest order, [`MergeError::Write`] for output problems, and
/// [`MergeError::Collision`] when two rows share a digest but differ.
pub fn merge_compact<W: Write>(
    first: &Path,
    second: &Path,
    mut writer: W,
) -> Result<MergeSummary, MergeError> {
    // The context drives serialization: it omits a `closing_whitespace` / `url` field that matches
    // the WTJ default or the inferred URL.
    let context = validation_context();

    let mut first = SnapshotLines::open(first)?;
    let mut second = SnapshotLines::open(second)?;
    let mut summary = MergeSummary::default();

    loop {
        // The next digest to emit is the smaller of the two streams' heads (both ascending).
        let digest = match (first.peek_digest()?, second.peek_digest()?) {
            (None, None) => break,
            (Some(first), Some(second)) => first.min(second),
            (Some(digest), None) | (None, Some(digest)) => digest,
        };

        // Drain every row sharing this digest from both streams (equal digests are consecutive in a
        // sorted file), keeping only one representative in memory.
        let mut representative: Option<CompactSnapshot<'static>> = None;
        let mut group = 0usize;
        for stream in [&mut first, &mut second] {
            while stream.peek_digest()? == Some(digest) {
                if let Some(snapshot) = stream.take()? {
                    group += 1;
                    match &representative {
                        None => representative = Some(snapshot),
                        // Same digest but a differing row is a collision.
                        Some(rep) if *rep != snapshot => {
                            return Err(MergeError::Collision { digest });
                        }
                        Some(_) => {}
                    }
                }
            }
        }

        // `representative` is set whenever the digest matched a row (i.e. always, since it came
        // from one of the streams).
        if let Some(representative) = representative {
            let duplicates = group - 1;
            if duplicates > 0 {
                summary.duplicates += duplicates;
                log::warn!("Dropped {duplicates} duplicate row(s) for digest {digest}");
            }

            writeln!(writer, "{}", representative.display(&context)).map_err(MergeError::Write)?;
            summary.read += group;
            summary.written += 1;
        }
    }

    Ok(summary)
}

/// A streaming, peekable reader over a zstd-compressed compact file that parses one snapshot at a
/// time and verifies the file is sorted by digest (failing with [`MergeError::NotSorted`]
/// otherwise).
struct SnapshotLines {
    lines: std::io::Lines<Box<dyn BufRead>>,
    path: PathBuf,
    line: usize,
    last_digest: Option<Sha1Digest>,
    peeked: Option<CompactSnapshot<'static>>,
}

impl SnapshotLines {
    fn open(path: &Path) -> Result<Self, MergeError> {
        let read = |source| MergeError::Read {
            path: path.to_path_buf(),
            source,
        };
        let decoder = zstd::Decoder::new(File::open(path).map_err(read)?).map_err(read)?;
        let reader: Box<dyn BufRead> = Box::new(BufReader::new(decoder));

        Ok(Self {
            lines: reader.lines(),
            path: path.to_path_buf(),
            line: 0,
            last_digest: None,
            peeked: None,
        })
    }

    /// Ensures `peeked` holds the next snapshot (or stays `None` at end of file), verifying that
    /// each line's digest is not smaller than the previous one.
    fn fill(&mut self) -> Result<(), MergeError> {
        while self.peeked.is_none() {
            let Some(line) = self.lines.next() else {
                return Ok(());
            };
            let line = line.map_err(|source| MergeError::Read {
                path: self.path.clone(),
                source,
            })?;
            self.line += 1;
            if line.trim().is_empty() {
                continue;
            }

            let snapshot = CompactSnapshot::parse(&line)
                .map_err(|source| MergeError::Parse {
                    path: self.path.clone(),
                    line: self.line,
                    source,
                })?
                .into_static();

            if self.last_digest.is_some_and(|last| snapshot.digest < last) {
                return Err(MergeError::NotSorted {
                    path: self.path.clone(),
                    line: self.line,
                });
            }
            self.last_digest = Some(snapshot.digest);
            self.peeked = Some(snapshot);
        }
        Ok(())
    }

    /// The digest of the next snapshot, or `None` at end of file.
    fn peek_digest(&mut self) -> Result<Option<Sha1Digest>, MergeError> {
        self.fill()?;
        Ok(self.peeked.as_ref().map(|snapshot| snapshot.digest))
    }

    /// Consumes and returns the next snapshot, or `None` at end of file.
    fn take(&mut self) -> Result<Option<CompactSnapshot<'static>>, MergeError> {
        self.fill()?;
        Ok(self.peeked.take())
    }
}

#[cfg(test)]
mod tests {
    // Status IDs are long opaque integers; digit separators do not aid readability here.
    #![allow(clippy::unreadable_literal)]

    use super::*;

    /// The curated Truth Social snapshot corpus, shared with the `truthsocial` crate's fixtures.
    /// Each file is named by the uppercase Base32 SHA-1 digest of its original bytes (so the
    /// filenames sort in digest order) and holds one snapshot's `StatusContent` payload; some are
    /// gzip-compressed (magic `1f 8b`).
    const CURATED_SNAPSHOTS_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/data/wbm/snapshots"
    );

    /// Decodes a curated snapshot payload to its JSON text, transparently gunzipping when the gzip
    /// magic bytes are present.
    fn decode_snapshot(bytes: &[u8]) -> String {
        use std::io::Read as _;
        if bytes.starts_with(&[0x1f, 0x8b]) {
            let mut text = String::new();
            flate2::read::GzDecoder::new(bytes)
                .read_to_string(&mut text)
                .expect("gzip snapshot decompresses to text");
            text
        } else {
            String::from_utf8(bytes.to_vec()).expect("snapshot is UTF-8 JSON")
        }
    }

    /// Builds compact snapshot lines (`{"digest":..,"content":<single status>}`) from the curated
    /// single-status snapshots, in ascending digest order (the filenames are the Base32 digests, so
    /// a lexical sort matches digest order). Array-content snapshots are skipped so every line's
    /// content is a single status object, giving the compact-file tests a predictable status count.
    fn curated_single_status_lines() -> Vec<String> {
        let mut paths = std::fs::read_dir(CURATED_SNAPSHOTS_DIR)
            .expect("curated snapshots directory exists")
            .map(|entry| entry.expect("readable directory entry").path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        // Sort by the decoded digest, not the filename: Base32's digits (`2`–`7`) encode the high
        // values 26–31 yet sort below `A` in ASCII, so a lexical filename sort is not digest order,
        // which the streaming merge requires.
        paths.sort_by_key(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .expect("filename is valid UTF-8")
                .parse::<Sha1Digest>()
                .expect("filename is a Base32 SHA-1 digest")
        });

        paths
            .iter()
            .filter_map(|path| {
                let digest = path.file_name()?.to_str()?;
                let content = decode_snapshot(&std::fs::read(path).expect("read snapshot"));
                let value: serde_json::Value =
                    serde_json::from_str(&content).expect("snapshot is JSON");
                // Re-serialize compactly so each line is a single line of valid JSON with the
                // `digest` field first and `content` last, as the exact-bytes parser requires.
                value.is_object().then(|| {
                    format!(
                        r#"{{"digest":"{digest}","content":{}}}"#,
                        serde_json::to_string(&value).expect("re-serialize content")
                    )
                })
            })
            .collect()
    }

    /// A curated single-status snapshot wrapped as a compact envelope deserializes as a
    /// [`WtjSnapshot`], exercising the core `StatusContent` schema of its content.
    #[test]
    fn parses_snapshot_envelope() {
        let lines = curated_single_status_lines();
        let line = lines
            .first()
            .expect("at least one curated single-status snapshot");
        let snapshot: WtjSnapshot<'_> = serde_json::from_str(line).expect("snapshot deserializes");
        assert!(matches!(snapshot.content, StatusContent::Single(_)));
    }

    /// The context infers a status's canonical URL (via its CEL query) from its content.
    #[test]
    fn context_infers_status_url() {
        // `5QIKZOW6…` is the first curated snapshot in digest order; its status id is embedded in
        // the inferred URL.
        let json = decode_snapshot(include_bytes!(
            "../../core/tests/data/wbm/snapshots/5QIKZOW6KFPXX3Q2UNR4HORDC2IPHFSZ"
        ));

        let url = validation_context().infer_url(&json);
        assert_eq!(
            url.as_deref(),
            Some("https://truthsocial.com/api/v1/statuses/114433942825461076")
        );
    }

    /// WTJ snapshots close each line with a single newline.
    #[test]
    fn default_closing_whitespace_is_newline() {
        assert_eq!(validation_context().default_closing_whitespace(), &['\n']);
    }

    /// Every gzip-compressed curated snapshot is recognised by the gzip crate, stored as an
    /// unprocessed snapshot under [`validation_context`], and validates against the digest of its
    /// original compressed bytes, the regression guard that the external gzip codec still
    /// reproduces Truth Social's archives byte-for-byte (Go-flate, zlib, and zlib-ng).
    #[test]
    fn validation_context_reproduces_gzip_snapshots() {
        use archivindex_wbm::digest::Sha1Digest;
        use archivindex_wbm_json::format::Format;
        use archivindex_wbm_json_gzip::{FORMAT, GzipParams};

        let context = validation_context();
        let format_name = Format::from(FORMAT);
        let mut hasher = sha1::Sha1::default();

        let mut count: usize = 0;
        let mut compressors = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(CURATED_SNAPSHOTS_DIR).expect("read curated snapshots") {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).expect("read snapshot");
            // The curated corpus stores original response bytes; only the gzip-compressed ones
            // (magic `1f 8b`) exercise the gzip codec.
            if !bytes.starts_with(&[0x1f, 0x8b]) {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();

            let params = GzipParams::infer(&bytes)
                .unwrap_or_else(|| panic!("{name}: no gzip parameters inferred"));
            compressors.insert(params.compressor);

            // Build the snapshot the way `compact` does: decode under the gzip format, then attach
            // the inferred reproduction metadata.
            let mut snapshot = context
                .unprocessed_snapshot(&format_name, &bytes)
                .unwrap_or_else(|error| panic!("{name}: unprocessed_snapshot: {error}"));
            snapshot.format.metadata = params.metadata();

            assert_eq!(
                snapshot.digest,
                Sha1Digest::compute(&bytes),
                "{name}: digest"
            );
            assert_eq!(
                context.verify(&snapshot, &mut hasher),
                Ok(()),
                "{name}: verify"
            );
            count += 1;
        }

        // Every gzip snapshot present has been reproduced byte-for-byte above. The curated corpus
        // is a small sample, so the full three-compressor spread it once guarded can only be judged
        // against the whole archive; note which compressors this sample happened to cover.
        assert!(
            count > 0,
            "no gzip snapshots found in {CURATED_SNAPSHOTS_DIR}"
        );
        let _ = writeln!(
            std::io::stderr(),
            "reproduced {count} gzip curated snapshot(s); compressors seen: {compressors:?}"
        );
    }

    /// Every line of a compact snapshot file (built from the curated single-status snapshots)
    /// parses into a status, in ascending digest order.
    #[test]
    fn read_statuses_from_compact_file() {
        let lines = curated_single_status_lines();
        let refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
        let file = compact_file(&refs);

        let statuses = read_statuses(file.path())
            .expect("open compact snapshot file")
            .collect::<Result<Vec<_>, _>>()
            .expect("every line parses into a status");

        // One status per curated single-status snapshot; the first is `AAACL3SP…` in digest order.
        assert_eq!(statuses.len(), lines.len());
        assert_eq!(statuses[0].id, 115864791948135518);
    }

    /// `read_statuses` flattens array content: a snapshot whose content is an array of statuses
    /// yields each of them. The array form previously failed to parse as a single `Status` (a
    /// derived struct also accepts sequence form, so the array was read as a positional status
    /// whose first field (the integer-string `id`) received the first element, an object).
    #[test]
    fn read_statuses_flattens_array_content() {
        // Seed from a curated single-status line, then reuse its content as an array of two.
        let lines = curated_single_status_lines();
        let seed = lines
            .first()
            .expect("at least one curated single-status snapshot");
        let single: serde_json::Value = serde_json::from_str(seed).expect("parse seed line");
        let status = single["content"].clone();
        let array_line = serde_json::json!({
            "digest": single["digest"],
            "content": [status.clone(), status],
        })
        .to_string();

        let file = compact_file(&[seed, &array_line]);
        let statuses = read_statuses(file.path())
            .expect("open compact snapshot file")
            .collect::<Result<Vec<_>, _>>()
            .expect("array content flattens into statuses");

        // One status from the single-status line, two from the array line.
        assert_eq!(statuses.len(), 3);
    }

    /// Writes the given lines to a fresh zstd-compressed temp file.
    fn compact_file(lines: &[&str]) -> tempfile::NamedTempFile {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        let mut encoder = zstd::Encoder::new(temp.as_file(), 0).expect("zstd encoder");
        for line in lines {
            writeln!(encoder, "{line}").expect("write line");
        }
        encoder.finish().expect("finish zstd");
        temp
    }

    /// Merging a compact file (built from the curated single-status snapshots) with itself drops
    /// every row as a duplicate and emits the unique rows in ascending digest order.
    #[test]
    fn merge_dedups_and_orders() {
        let lines = curated_single_status_lines();
        let refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
        let file = compact_file(&refs);
        let n = lines.len();

        let mut output = Vec::new();
        let summary = merge_compact(file.path(), file.path(), &mut output).expect("merge succeeds");

        assert_eq!(summary.read, 2 * n);
        assert_eq!(summary.written, n);
        assert_eq!(summary.duplicates, n);
        assert_eq!(summary.read, summary.written + summary.duplicates);

        // The output is itself a valid compact file whose digests ascend.
        let text = std::str::from_utf8(&output).expect("utf-8 output");
        let mut previous: Option<Sha1Digest> = None;
        let mut count = 0;
        for line in text.lines() {
            let snapshot = ExactSnapshot::parse(line).expect("parse output line");
            if let Some(previous) = previous {
                assert!(snapshot.digest >= previous, "output not in digest order");
            }
            previous = Some(snapshot.digest);
            count += 1;
        }
        assert_eq!(count, n);
    }

    /// Two rows that share a digest but differ in content are a collision.
    #[test]
    fn merge_detects_collision() {
        let first =
            compact_file(&[r#"{"digest":"AAAA3HVFIBJARGQ4ISEHROP6XWNULWTC","content":{"x":1}}"#]);
        let second =
            compact_file(&[r#"{"digest":"AAAA3HVFIBJARGQ4ISEHROP6XWNULWTC","content":{"x":2}}"#]);

        let mut output = Vec::new();
        let result = merge_compact(first.path(), second.path(), &mut output);

        assert!(matches!(result, Err(MergeError::Collision { .. })));
    }

    /// A file whose digests are not ascending is rejected with [`MergeError::NotSorted`] rather
    /// than silently mis-merging (the streaming merge requires sorted input).
    #[test]
    fn merge_detects_unsorted_input() {
        // `BAAA…` sorts after `AAAA…` by digest bytes, so this file is descending.
        let unsorted = compact_file(&[
            r#"{"digest":"BAAA3HVFIBJARGQ4ISEHROP6XWNULWTC","content":{"x":1}}"#,
            r#"{"digest":"AAAA3HVFIBJARGQ4ISEHROP6XWNULWTC","content":{"x":2}}"#,
        ]);
        let sorted =
            compact_file(&[r#"{"digest":"AAAA3HVFIBJARGQ4ISEHROP6XWNULWTC","content":{"x":3}}"#]);

        let mut output = Vec::new();
        let result = merge_compact(unsorted.path(), sorted.path(), &mut output);

        assert!(
            matches!(result, Err(MergeError::NotSorted { line: 2, .. })),
            "expected NotSorted, got {result:?}"
        );
    }
}
