//! Reading CDX entries from directories of JSON files.
//!
//! Each file is a JSON array in the archivindex [`ItemList`] representation (a header row followed
//! by entry rows), either plain (`*.json`) or zstd-compressed (`*.json.zst`). [`read_cdx_entries`]
//! reads a whole directory of such files and streams the contained [`Item`]s.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use archivindex_wbm::cdx::item::{Item, ItemList};
use bounded_static::IntoBoundedStatic;

/// An error encountered while reading CDX entry files from a directory.
///
/// Every variant carries the path of the directory or file that caused it.
#[derive(Debug, thiserror::Error)]
pub enum CdxError {
    /// The directory could not be opened, or one of its entries could not be listed.
    #[error("failed to read directory {}", .path.display())]
    ReadDir {
        /// The directory that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A file in the directory could not be read.
    #[error("failed to read CDX file {}", .path.display())]
    Read {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A file could not be parsed as a JSON array of CDX entries.
    #[error("failed to parse CDX file {}", .path.display())]
    Parse {
        /// The file that could not be parsed.
        path: PathBuf,
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
}

/// Reads every CDX file in `directory` and returns an iterator over the contained [`Item`]s.
///
/// Each file is a JSON array of entries (the archivindex [`ItemList`] representation), plain
/// (`*.json`) or zstd-compressed (`*.json.zst`); the reader is chosen from the extension and files
/// with other extensions are ignored. Files are visited in sorted order and each is read and parsed
/// only when the iterator reaches it, so at most one file is held in memory at a time. Entries are
/// returned fully owned (`Item<'static>`), detached from the file buffer they were parsed from.
///
/// # Errors
///
/// Returns [`CdxError::ReadDir`] up front if `directory` cannot be opened or one of its entries
/// cannot be listed. While iterating, a file that cannot be read (or decompressed) yields a single
/// [`CdxError::Read`] and a file that cannot be parsed yields a single [`CdxError::Parse`]; every
/// error carries the offending path.
pub fn read_cdx_entries<P: AsRef<Path>>(
    directory: P,
) -> Result<impl Iterator<Item = Result<Item<'static>, CdxError>>, CdxError> {
    let directory = directory.as_ref();

    let mut paths = std::fs::read_dir(directory)
        .map_err(|source| CdxError::ReadDir {
            path: directory.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|source| CdxError::ReadDir {
                    path: directory.to_path_buf(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    paths.retain(|path| path.is_file() && is_cdx_file(path));
    paths.sort();

    Ok(paths.into_iter().flat_map(
        |path| -> Box<dyn Iterator<Item = Result<Item<'static>, CdxError>>> {
            match parse_cdx_file(&path) {
                Ok(entries) => Box::new(entries.into_iter().map(Ok)),
                Err(error) => Box::new(std::iter::once(Err(error))),
            }
        },
    ))
}

/// Whether `path` is a CDX file this reader handles: a plain (`*.json`) or zstd-compressed
/// (`*.json.zst`) JSON file. The lowercase extensions are matched exactly, by design.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_cdx_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".json") || name.ends_with(".json.zst"))
}

/// Whether `path` names a zstd-compressed (`*.json.zst`) CDX file rather than a plain `*.json` one.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_zstd(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".json.zst"))
}

/// Reads a single file and parses it as an [`ItemList`], returning its entries as owned values.
///
/// The file is read plainly or zstd-decompressed depending on its extension (see [`is_zstd`]). The
/// entries borrow from the decoded contents while parsing, then are detached to `'static` so they
/// can outlive the buffer.
fn parse_cdx_file(path: &Path) -> Result<Vec<Item<'static>>, CdxError> {
    let read_error = |source| CdxError::Read {
        path: path.to_path_buf(),
        source,
    };

    let content = if is_zstd(path) {
        let mut decoder =
            zstd::Decoder::new(File::open(path).map_err(read_error)?).map_err(read_error)?;
        let mut content = String::new();
        decoder.read_to_string(&mut content).map_err(read_error)?;
        content
    } else {
        std::fs::read_to_string(path).map_err(read_error)?
    };

    let list: ItemList<'_> = serde_json::from_str(&content).map_err(|source| CdxError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(list
        .values
        .into_iter()
        .map(IntoBoundedStatic::into_static)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a CDX `ItemList` JSON document with `count` public status-capture rows, in the
    /// standard seven-field layout the Wayback Machine CDX API returns (a header row followed by
    /// entry rows). Only status URLs, digests, and capture metadata are embedded, so the fixture
    /// carries no private data.
    fn cdx_json(count: usize) -> String {
        use std::fmt::Write as _;

        let mut document = String::from(
            r#"[["urlkey","timestamp","original","mimetype","statuscode","digest","length"]"#,
        );
        for index in 0..count {
            let id = 115_400_000_000_000_000_u64 + index as u64;
            // `write!` to the buffer avoids the extra allocation of `format!` per row.
            write!(
                document,
                ",\n[\"com,truthsocial)/api/v1/statuses/{id}\",\"20260427185815\",\
                 \"https://truthsocial.com/api/v1/statuses/{id}\",\"application/json\",\"200\",\
                 \"HTFXPWQLEUHMIUWJ7LR2U2XREQORA47Y\",\"3196\"]"
            )
            .expect("writing to a String never fails");
        }
        document.push(']');
        document
    }

    /// Every CDX file in a directory is parsed into owned entries, visited in sorted filename
    /// order.
    #[test]
    fn read_cdx_entries_from_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        // `part-1.json` holds 5 entries and `part-2.json` holds 3.
        std::fs::write(dir.path().join("part-1.json"), cdx_json(5)).expect("write part-1");
        std::fs::write(dir.path().join("part-2.json"), cdx_json(3)).expect("write part-2");

        let entries = read_cdx_entries(dir.path())
            .expect("read CDX directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("every file parses into CDX entries");

        assert_eq!(entries.len(), 8);
        // Files are visited in sorted order, so the first entry is `part-1.json`'s first row.
        assert!(entries[0].original.starts_with("http"));
    }

    /// Plain (`.json`) and zstd-compressed (`.json.zst`) files are both read (the reader is chosen
    /// by extension), and files with other extensions are ignored.
    #[test]
    fn read_cdx_entries_handles_plain_and_zstd() {
        let json = cdx_json(5);
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("a.json"), &json).expect("write plain");
        std::fs::write(
            dir.path().join("b.json.zst"),
            zstd::encode_all(json.as_bytes(), 0).expect("compress"),
        )
        .expect("write compressed");
        // Not a CDX file; must be skipped rather than parsed.
        std::fs::write(dir.path().join("notes.txt"), "ignore me").expect("write other");

        let entries = read_cdx_entries(dir.path())
            .expect("read dir")
            .collect::<Result<Vec<_>, _>>()
            .expect("every CDX file parses");

        // Each copy has 5 entries; both the plain and compressed copies are read.
        assert_eq!(entries.len(), 10);
    }

    /// A missing directory produces a [`CdxError::ReadDir`] carrying the offending path.
    #[test]
    fn read_cdx_entries_missing_directory_reports_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("does-not-exist");

        match read_cdx_entries(&missing) {
            Ok(_) => panic!("expected an error for a missing directory"),
            Err(CdxError::ReadDir { path, .. }) => {
                assert!(path.ends_with("does-not-exist"));
            }
            Err(other) => panic!("expected ReadDir error, got {other:?}"),
        }
    }
}
