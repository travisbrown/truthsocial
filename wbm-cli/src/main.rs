//! CLI tool for validating Truth Social status JSON files.

use archivindex_wbm::cdx::item::Item;
use archivindex_wbm::digest::Sha1Digest;
use archivindex_wbm_json::format::FormatInfo;
use archivindex_wbm_json_gzip::GzipParams;
use archivindex_wbm_json_processing::io::read::SnapshotReader;
use archivindex_wbm_json_processing::process::compact::{CompactConfig, Partition};
use clap::Parser;
use cli_helpers::prelude::*;
use rand::RngExt;
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use truthsocial::model::StatusContent;

pub mod cdx;

/// Base32 SHA-1 digest of an empty response body.
const EMPTY_BODY_DIGEST: &str = "3I42H3S6NNFQ2MSVX7XZKYAYSCX5QBYJ";
/// Base32 SHA-1 digest of Truth Social's `{"error":"record not found"}` body, returned for deleted
/// or otherwise unavailable statuses.
const NOT_FOUND_BODY_DIGEST: &str = "P5TTLDT2OWSDBLWJ4JZRW26NZ3Q7XVQS";

fn main() -> Result<(), Error> {
    let opts = Opts::parse();
    opts.verbose.init_logging()?;

    match opts.command {
        Command::ValidateFiles { path } => {
            validate_statuses(&path)?;
        }
        Command::Missing {
            cdx,
            compact,
            limit,
        } => {
            print_missing_cdx(&cdx, &compact, limit)?;
        }
        Command::Merge { first, second } => {
            merge_compact_files(&first, &second)?;
        }
        Command::Compact {
            cdx,
            data,
            invalid_db,
            output,
            summary,
            level,
            skip_unresolved,
        } => {
            compact(
                &cdx,
                &data,
                &invalid_db,
                &output,
                &summary,
                level,
                skip_unresolved,
            )?;
        }
        Command::Extract { base } => {
            let rows = cdx::read_all(base)?;

            for row in rows {
                println!(
                    "https://truthsocial.com/api/v1/statuses/{},{},{}",
                    row.id, row.timestamp, row.digest
                );
            }
        }
        Command::Next {
            digests,
            cdx,
            count,
        } => {
            let digests = BufReader::new(File::open(digests)?)
                .lines()
                .filter_map(|result| {
                    result
                        .map_err(Error::from)
                        .and_then(|line| {
                            // Drop the two "no real content" digests (empty and not-found bodies)
                            // from the already-captured set, so a status whose only capture is one
                            // of them is still surfaced as missing and re-fetched.
                            if line == EMPTY_BODY_DIGEST || line == NOT_FOUND_BODY_DIGEST {
                                Ok(None)
                            } else {
                                line.parse::<Sha1Digest>()
                                    .map_err(|error| Error::from(std::io::Error::other(error)))
                                    .map(Some)
                            }
                        })
                        .map_or_else(|error| Some(Err(error)), |value| value.map(Ok))
                })
                .collect::<Result<BTreeSet<_>, Error>>()?;

            log::info!("Read {} digests", digests.len());

            let rows = cdx::read_compressed_csv(cdx)?;

            log::info!("Read {} CDX rows", rows.len());

            for row in rows
                .iter()
                .rev()
                .filter(|row| !digests.contains(&row.digest))
                .take(count)
            {
                println!(
                    "https://truthsocial.com/api/v1/statuses/{},{},{}",
                    row.id, row.timestamp, row.digest
                );
            }
        }
        Command::Pack {
            data,
            invalid_db,
            output,
            level,
        } => {
            let context = truthsocial_wbm::validation_context();
            let summary = archivindex_wbm_json_processing::process::pack::pack(
                &data,
                Some(&invalid_db),
                &output,
                level,
                &context,
                // Gzip archives carry their inferred reproduction parameters in the `format`
                // object; everything else (plain UTF-8 text included) uses the default format.
                |bytes| GzipParams::infer(bytes).map(|params| params.format_info()),
            )?;

            log::info!(
                "Packed: {} written ({} with an expected digest), {} skipped",
                summary.written_count,
                summary.expected_digest_count,
                summary.skipped_count
            );

            println!("{}", serde_json::json!(summary));
        }
        Command::Enhance {
            input,
            metadata_db,
            invalid_db,
            output,
            level,
            batch_size,
        } => {
            let context = truthsocial_wbm::validation_context();
            let metadata = archivindex_wbm_cdx_index::metadata::MetadataDb::open(&metadata_db)?;
            let summary = archivindex_wbm_json_processing::process::enhance::enhance(
                &input,
                &invalid_db,
                &output,
                level,
                batch_size,
                &context,
                |digests| metadata.multi_get(digests),
            )?;

            log::info!(
                "Enhanced: {} read, {} enhanced, {} already enhanced, {} unmatched",
                summary.read_count,
                summary.enhanced_count,
                summary.already_enhanced_count,
                summary.unmatched_count
            );

            println!("{}", serde_json::json!(summary));
        }
        Command::Validate { input } => {
            let context = truthsocial_wbm::validation_context();
            validate_compact(&input, &context)?;
        }
    }

    Ok(())
}

/// Reads all files in a directory and attempts to deserialize them as status objects.
///
/// Prints errors for each file that fails to parse, then prints a summary of successful vs
/// unsuccessful parse counts.
fn validate_statuses(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();
    let mut success_count: u64 = 0;
    let mut error_count: u64 = 0;

    // Read directory entries, collecting into a `Vec` to handle errors upfront and avoid holding
    // the directory iterator open during processing.
    let mut paths = fs::read_dir(path)
        .map_err(|e| Error::ReadDir(path.to_path_buf(), e))?
        .map(|result| {
            result
                .map_err(|e| Error::DirEntry(path.to_path_buf(), e))
                .and_then(|entry| {
                    let path = entry.path();
                    let metadata = path
                        .metadata()
                        .map_err(|e| Error::DirEntry(path.to_path_buf(), e))?;
                    let created = metadata
                        .created()
                        .map_err(|e| Error::DirEntry(path.to_path_buf(), e))?;

                    Ok((path, created))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    log::info!("Parsing {} files", paths.len());

    paths.sort_by_key(|(_, created)| std::cmp::Reverse(*created));

    for (file_path, _) in paths {
        // Skip directories, only process files.
        if file_path.is_dir() {
            continue;
        }

        match validate_single_file(&file_path) {
            Ok(()) => {
                success_count += 1;
            }
            Err(e) => {
                // Undecodable files (binary that is neither UTF-8 nor gzip) are common; count but
                // don't log them.
                if !e.is_decode_error() {
                    log::warn!("{}: {}", file_path.display(), e);
                }
                error_count += 1;
            }
        }
    }

    log::info!("Parsed {success_count} files successfully, {error_count} failed");

    Ok(())
}

/// Validates a compact snapshot file: checks digests, schema, ordering, and metadata consistency
/// (via archivindex), then validates each snapshot's content against the Truth Social schema.
fn validate_compact(
    path: impl AsRef<Path>,
    context: &archivindex_wbm_json::context::Context,
) -> Result<(), Error> {
    // First, run the archivindex check (digests, WTJ schema, metadata).
    let check_summary =
        archivindex_wbm_json_processing::process::check::check(path.as_ref(), context)?;

    log::info!(
        "Checked {} lines: {} valid digests, {} schema errors, {} digest mismatches, \
         {} missing timestamps, {} with a URL but no timestamp",
        check_summary.line_count,
        check_summary.valid_digest_count,
        check_summary.schema_errors.len(),
        check_summary.digest_mismatches.len(),
        check_summary.missing_timestamp_count,
        check_summary.url_without_timestamp.len()
    );

    if !check_summary.is_successful() {
        log::warn!("WTJ validation found problems");
    }

    // Now validate each snapshot's content against the Truth Social schema.
    let content_validation = validate_content(path.as_ref())?;

    log::info!(
        "Validated {} snapshot contents: {} valid, {} schema errors",
        content_validation.total,
        content_validation.valid,
        content_validation.schema_errors.len()
    );

    if !content_validation.schema_errors.is_empty() {
        log::warn!(
            "Content validation found {} schema errors",
            content_validation.schema_errors.len()
        );
    }

    // Output combined results.
    let is_successful =
        check_summary.is_successful() && content_validation.schema_errors.is_empty();
    let combined = serde_json::json!({
        "wtj_check": check_summary,
        "content_validation": content_validation,
        "successful": is_successful
    });
    println!("{}", combined);

    if !is_successful {
        log::warn!("Validation found problems (see the summary for details)");
    }

    Ok(())
}

/// Summary of Truth Social content validation.
#[derive(Serialize)]
struct ContentValidationSummary {
    total: usize,
    valid: usize,
    schema_errors: Vec<String>,
}

/// Validates each snapshot's content against the Truth Social `StatusContent` schema.
fn validate_content(path: impl AsRef<Path>) -> Result<ContentValidationSummary, Error> {
    let mut total = 0;
    let mut valid = 0;
    let mut schema_errors = Vec::new();

    let reader = archivindex_wbm_json_processing::io::read::SnapshotReader::open(path.as_ref())?;

    for result in reader {
        total += 1;
        match result {
            Ok(snapshot) => {
                // Parse content as `StatusContent` (single status or array).
                match serde_json::from_str::<StatusContent>(snapshot.content.as_str()) {
                    Ok(_) => valid += 1,
                    Err(error) => {
                        schema_errors.push(format!("digest {}: {}", snapshot.digest, error));
                    }
                }
            }
            Err(error) => {
                schema_errors.push(format!("read error: {}", error));
            }
        }
    }

    Ok(ContentValidationSummary {
        total,
        valid,
        schema_errors,
    })
}

/// Validates a single file by reading and deserializing it as a status.
///
/// The file may be raw status JSON or a gzip archive of it (detected by the gzip magic and
/// decompressed); either way the content is parsed with the core [`StatusContent`] schema (a single
/// status or an array of them).
fn validate_single_file(path: impl AsRef<Path>) -> Result<(), FileError> {
    let bytes = fs::read(path.as_ref()).map_err(FileError::Read)?;

    let content = if bytes.starts_with(&[0x1f, 0x8b]) {
        archivindex_wbm_json_gzip::decompress(&bytes).ok_or(FileError::Decode)?
    } else {
        String::from_utf8(bytes).map_err(|_| FileError::Decode)?
    };

    let _content: StatusContent = serde_json::from_str(&content).map_err(FileError::Parse)?;
    Ok(())
}

/// Builds an index of the top-level status IDs and snapshot digests contained in a compact WTJ
/// snapshot file.
///
/// Each line is read through a [`SnapshotReader`] and its content parsed as [`StatusContent`]; only
/// the (`Copy`) status ID(s) and digest are retained, so the snapshots themselves are not held in
/// memory. A snapshot whose content is an array of statuses contributes every status ID it contains.
fn index_compact(path: &Path) -> Result<(HashSet<u64>, HashSet<Sha1Digest>), Error> {
    let mut ids = HashSet::new();
    let mut digests = HashSet::new();

    for snapshot in SnapshotReader::open(path)? {
        let snapshot = snapshot?;
        let content: StatusContent<'_> = serde_json::from_str(snapshot.content.as_str())?;
        for status in content.statuses() {
            ids.insert(status.id);
        }
        digests.insert(snapshot.digest);
    }

    Ok((ids, digests))
}

/// Prints, as headerless CSV, the CDX entries in `cdx_dir` whose status ID and content digest are
/// both absent from `compact`.
///
/// The compact file is indexed once (its top-level status IDs and digests), then every CDX entry is
/// checked against that index. An entry counts as missing only when neither its status ID (parsed
/// from the URL) nor its digest appears in the compact file. Each emitted row has three columns:
/// the URL, the Wayback-format timestamp, and the Base32 SHA-1 digest.
///
/// When `limit` is `Some(n)`, exactly `n` of the missing entries are selected uniformly at random
/// via reservoir sampling (or all of them if fewer than `n` match). When `limit` is `None`, every
/// missing entry is streamed out.
fn print_missing_cdx(cdx_dir: &Path, compact: &Path, limit: Option<usize>) -> Result<(), Error> {
    let (ids, digests) = index_compact(compact)?;
    log::info!(
        "Indexed {} status IDs and {} digests from the compact file",
        ids.len(),
        digests.len()
    );

    // CDX entries absent from the compact file. An entry counts as "present" only when it has a
    // recognizable status ID / valid digest that is actually indexed; otherwise it is missing.
    let missing =
        truthsocial_wbm::cdx::read_cdx_entries(cdx_dir)?.filter_map(|entry| match entry {
            Err(error) => Some(Err(error)),
            Ok(item) => {
                let id_present =
                    cdx::parse_status_id(&item.original).is_ok_and(|id| ids.contains(&id));
                let digest_present = item
                    .digest
                    .valid()
                    .is_some_and(|digest| digests.contains(&digest));

                if id_present || digest_present {
                    None
                } else {
                    Some(Ok(item))
                }
            }
        });

    let mut writer = csv::Writer::from_writer(std::io::stdout().lock());

    if let Some(limit) = limit {
        let mut sample = reservoir_sample(missing, limit)?;
        // Emit the random sample in a stable order: by URL, then timestamp.
        sample.sort_by(|a, b| {
            a.original
                .cmp(&b.original)
                .then_with(|| a.timestamp.cmp(&b.timestamp))
        });
        for item in &sample {
            write_cdx_row(&mut writer, item)?;
        }
    } else {
        for item in missing {
            write_cdx_row(&mut writer, &item?)?;
        }
    }

    writer.flush()?;

    Ok(())
}

/// Writes one CDX entry as a CSV row: URL, Wayback-format timestamp, and Base32 SHA-1 digest.
fn write_cdx_row<W: Write>(writer: &mut csv::Writer<W>, item: &Item<'_>) -> Result<(), Error> {
    writer.write_record([
        item.original.to_string(),
        item.timestamp.to_string(),
        item.digest.to_string(),
    ])?;

    Ok(())
}

/// Selects up to `k` items uniformly at random from a fallible stream using Algorithm R.
///
/// The reservoir grows to hold every item when the stream is shorter than `k`, so no more than
/// `min(k, stream length)` items are ever held. The first stream error aborts sampling.
fn reservoir_sample<T, E, I: Iterator<Item = Result<T, E>>>(
    items: I,
    k: usize,
) -> Result<Vec<T>, E> {
    let mut rng = rand::rng();
    let mut reservoir: Vec<T> = Vec::new();

    for (index, item) in items.enumerate() {
        let item = item?;
        if reservoir.len() < k {
            reservoir.push(item);
        } else {
            // Replace a random current member with probability k / (index + 1).
            let candidate = rng.random_range(0..=index);
            if candidate < k {
                reservoir[candidate] = item;
            }
        }
    }

    Ok(reservoir)
}

/// Merges two compact WTJ snapshot files and writes the result, ordered by SHA-1 digest, as
/// newline-delimited JSON to stdout.
///
/// Duplicate rows are logged and dropped; rows that share a digest but differ abort the merge. The
/// output is uncompressed so it can be inspected or piped (e.g. into `zstd`).
fn merge_compact_files(first: &Path, second: &Path) -> Result<(), Error> {
    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    let summary = truthsocial_wbm::merge_compact(first, second, &mut writer)?;
    writer.flush()?;

    log::info!(
        "Merged {} rows: wrote {} unique, dropped {} duplicates",
        summary.read,
        summary.written,
        summary.duplicates
    );

    Ok(())
}

/// Compacts a data directory of raw status JSON files (named by SHA-1 digest), enriched with CDX
/// metadata, into a single zstd-compressed NDJSON snapshot file, writing a JSON run summary.
///
/// Uses [`archivindex_wbm_json_processing::process::compact`] with a single output partition (the
/// Truth Social [`validation_context`](truthsocial_wbm::validation_context)). `invalid_db` is the
/// SQLite database of known-invalid digests consulted during CDX resolution.
fn compact(
    cdx: &Path,
    data: &Path,
    invalid_db: &Path,
    output: &Path,
    summary_path: &Path,
    level: u16,
    skip_unresolved: bool,
) -> Result<(), Error> {
    let context = truthsocial_wbm::validation_context();

    let summary = archivindex_wbm_json_processing::process::compact::compact(
        &[data],
        &[cdx],
        CompactConfig {
            // A single output partition; the discriminator always selects it.
            partitions: vec![Partition {
                key: (),
                output,
                context: &context,
            }],
            invalid_db,
            compression_level: level,
            skip_unresolved,
            // The CDX directory is a flat directory of index files, as before.
            cdx_recursive: false,
        },
        // Gzip archives carry their inferred reproduction parameters in the `format` object;
        // everything else (plain UTF-8 text included) uses the default format.
        |bytes, _resolution| {
            (
                (),
                GzipParams::infer(bytes)
                    .map_or_else(FormatInfo::default, |params| params.format_info()),
            )
        },
    )?;

    serde_json::to_writer_pretty(File::create(summary_path)?, &summary)?;

    log::info!(
        "Compacted: {} resolved, {} unresolved, {} skipped",
        summary.resolved_count,
        summary.unresolved_count,
        summary.skipped_count
    );

    Ok(())
}

/// Top-level application error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to read directory '{0}'")]
    ReadDir(PathBuf, #[source] std::io::Error),
    #[error("Failed to read directory entry in '{0}'")]
    DirEntry(PathBuf, #[source] std::io::Error),
    #[error("CLI argument reading error")]
    Args(#[from] cli_helpers::Error),
    #[error("CDX source directory reading error")]
    CdxSource(#[from] cdx::Error),
    #[error("CDX entry reading error")]
    WbmCdx(#[from] truthsocial_wbm::cdx::CdxError),
    #[error("compact file merge error")]
    Merge(#[from] truthsocial_wbm::MergeError),
    #[error("compact error")]
    Compact(#[from] archivindex_wbm_json_processing::process::compact::Error),
    #[error("pack error")]
    Pack(#[from] archivindex_wbm_json_processing::process::pack::Error),
    #[error("enhance error")]
    Enhance(
        #[from]
        archivindex_wbm_json_processing::process::enhance::Error<
            archivindex_wbm_cdx_index::metadata::Error,
        >,
    ),
    #[error("check error")]
    Check(#[from] archivindex_wbm_json_processing::process::check::Error),
    #[error("capture metadata database error")]
    Metadata(#[from] archivindex_wbm_cdx_index::metadata::Error),
    #[error("JSON parsing error")]
    Json(#[from] serde_json::Error),
    #[error("compact snapshot reading error")]
    Snapshot(#[from] archivindex_wbm_json::Error),
    #[error("CSV writing error")]
    Csv(#[from] csv::Error),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
}

/// Error for individual file processing.
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("read error: {0}")]
    Read(#[source] std::io::Error),
    #[error("could not decode as UTF-8 text or a gzip archive")]
    Decode,
    #[error("parse error: {0}")]
    Parse(#[source] serde_json::Error),
}

impl FileError {
    /// Returns true if the file could not be decoded as UTF-8 text or gzip (common for binary files
    /// that are not statuses).
    const fn is_decode_error(&self) -> bool {
        matches!(self, Self::Decode)
    }
}

#[derive(Debug, Parser)]
#[command(name = "truthsocial-wbm", version, author)]
struct Opts {
    #[clap(flatten)]
    verbose: Verbosity,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    /// Validate status JSON files in a directory
    ValidateFiles {
        /// Path to directory containing status JSON files
        path: PathBuf,
    },
    /// Print (as CSV) CDX entries whose status ID and digest are both absent from a compact file
    Missing {
        /// Path to a directory of CDX entry files (JSON arrays of CDX entries)
        #[clap(long)]
        cdx: PathBuf,
        /// Path to a zstd-compressed compact WTJ snapshot file
        #[clap(long)]
        compact: PathBuf,
        /// If set, randomly sample exactly this many matching entries (all of them if fewer exist)
        #[clap(long)]
        limit: Option<usize>,
    },
    /// Merge two compact WTJ snapshot files (in SHA-1 digest order) to stdout
    Merge {
        /// Path to the first compact file
        first: PathBuf,
        /// Path to the second compact file
        second: PathBuf,
    },
    /// Compact a data directory + CDX metadata into a zstd NDJSON snapshot file
    Compact {
        /// Directory of CDX JSON files (searched recursively)
        #[clap(long)]
        cdx: PathBuf,
        /// Directory of raw data files, each named by the SHA-1 digest of its bytes
        #[clap(long)]
        data: PathBuf,
        /// Path to the SQLite database of known-invalid digests
        #[clap(long)]
        invalid_db: PathBuf,
        /// Output path for the compacted zstd NDJSON file (must not already exist)
        #[clap(long)]
        output: PathBuf,
        /// Output path for the JSON run summary
        #[clap(long)]
        summary: PathBuf,
        /// Zstandard compression level
        #[clap(long, default_value = "14")]
        level: u16,
        /// Omit snapshots that have no CDX resolution from the output
        #[clap(long)]
        skip_unresolved: bool,
    },
    /// Extract deduplicated `(url, timestamp, digest)` rows from a directory of CDX files to stdout
    Extract {
        /// Directory of CDX entry files (plain `*.json` or zstd-compressed `*.json.zst`)
        base: PathBuf,
    },
    /// Print capture URLs from a CDX CSV whose digest is not in a file of already-captured digests
    Next {
        /// Path to a newline-delimited file of Base32 SHA-1 digests already captured
        #[clap(long)]
        digests: PathBuf,
        /// Path to a zstd-compressed CSV of `(url, timestamp, digest)` rows (from `extract`)
        #[clap(long)]
        cdx: PathBuf,
        /// Maximum number of URLs to print (newest first)
        #[clap(long, default_value = "1000000")]
        count: usize,
    },
    /// Pack a data directory of digest-named status files into a compact zstd NDJSON file, without
    /// CDX metadata (only the digest, the expected digest from the invalid-digest log, the format
    /// when non-default, and the content)
    Pack {
        /// Directories of raw data files, each named by the SHA-1 digest of its bytes
        #[clap(long)]
        data: Vec<PathBuf>,
        /// Path to the SQLite database of known-invalid digests
        #[clap(long)]
        invalid_db: PathBuf,
        /// Output path for the packed zstd NDJSON file (must not already exist)
        #[clap(long)]
        output: PathBuf,
        /// Zstandard compression level
        #[clap(long, default_value = "14")]
        level: u16,
    },
    /// Enhance a compact snapshot file with CDX metadata (timestamp, and a URL when the content
    /// does not infer it) from a digest-keyed capture metadata database
    Enhance {
        /// Path to a zstd-compressed compact WTJ snapshot file
        #[clap(long)]
        input: PathBuf,
        /// Path to the digest-keyed capture metadata database
        #[clap(long)]
        metadata_db: PathBuf,
        /// Path to the SQLite database of known-invalid digests
        #[clap(long)]
        invalid_db: PathBuf,
        /// Output path for the enhanced zstd NDJSON file (must not already exist)
        #[clap(long)]
        output: PathBuf,
        /// Zstandard compression level
        #[clap(long, default_value = "14")]
        level: u16,
        /// Number of snapshots buffered per capture lookup batch
        #[clap(long, default_value = "1024")]
        batch_size: NonZeroUsize,
    },
    /// Validate a compact snapshot file: schema, digests, ordering, metadata consistency, and Truth
    /// Social content schema compliance
    Validate {
        /// Path to a zstd-compressed compact WTJ snapshot file
        #[clap(long)]
        input: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reservoir sampling keeps exactly `min(k, stream length)` distinct items.
    #[test]
    fn reservoir_sample_size_and_membership() {
        let sample = reservoir_sample((0..100).map(Ok::<u32, ()>), 10).unwrap();
        assert_eq!(sample.len(), 10);
        assert!(sample.iter().all(|value| *value < 100));
        let distinct = sample.iter().collect::<HashSet<_>>();
        assert_eq!(distinct.len(), 10, "sampled items must be distinct");

        // A limit larger than the stream returns every item.
        let all = reservoir_sample((0..5).map(Ok::<u32, ()>), 10).unwrap();
        assert_eq!(all.len(), 5);

        // A zero limit returns nothing.
        let none = reservoir_sample((0..100).map(Ok::<u32, ()>), 0).unwrap();
        assert!(none.is_empty());
    }

    /// The first error in the stream aborts sampling.
    #[test]
    fn reservoir_sample_propagates_error() {
        let result = reservoir_sample([Ok(1), Err("boom"), Ok(3)].into_iter(), 5);
        assert_eq!(result, Err("boom"));
    }
}
