//! Shared helpers for the fixture-coverage integration tests.
//!
//! Each `tests/*.rs` file is its own compilation unit, so code shared between them lives here and is
//! pulled in with `mod common;`. Living under `tests/common/` (rather than as a top-level
//! `tests/common.rs`) keeps Cargo from treating it as its own test binary.

use std::collections::BTreeSet;

/// Assert that the `<name>` keys registered in `fixtures` exactly match the filenames in `dir`.
///
/// Every fixture list is paired with an on-disk directory; this catches both directions of drift: a
/// file added to disk but never registered (so it goes untested) and a fixture registered but
/// missing from disk. `dir` is an absolute path (the caller builds it from `CARGO_MANIFEST_DIR`).
///
/// # Arguments
///
/// * `dir`: Absolute path to the directory whose `.json` files back `fixtures`.
/// * `fixtures`: The compile-time `(filename, contents)` entries under test.
///
/// # Panics
///
/// Panics if `dir` cannot be read, if any entry name is not valid UTF-8, or if the registered names
/// and the on-disk names diverge in either direction.
pub fn assert_directory_matches(dir: &str, fixtures: &[(&str, &str)]) {
    let enumerated = fixtures
        .iter()
        .map(|&(name, _)| name)
        .collect::<BTreeSet<_>>();
    let on_disk = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{dir} is readable: {error}"))
        .map(|entry| {
            entry
                .expect("readable directory entry")
                .file_name()
                .into_string()
                .expect("valid UTF-8 filename")
        })
        .collect::<BTreeSet<_>>();
    let on_disk = on_disk.iter().map(String::as_str).collect::<BTreeSet<_>>();

    let unregistered = on_disk.difference(&enumerated).collect::<Vec<_>>();
    assert!(
        unregistered.is_empty(),
        "files present in {dir} but not registered: {unregistered:?}"
    );
    let missing = enumerated.difference(&on_disk).collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "fixtures registered but missing from {dir}: {missing:?}"
    );
}
