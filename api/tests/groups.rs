//! Schema-coverage guard for the curated single-group lookup fixtures.
//!
//! Every file in `tests/data/groups/by-id/` is a real `GET /api/v1/groups/:id` response, named
//! `<group-id>.json`. Fixtures are embedded at compile time via [`FIXTURES`] and asserted to
//! deserialize as [`Group`], exercising the fields that appear only in the single-group view (the
//! [`member_avatars`](Group::member_avatars) preview). A second test keeps [`FIXTURES`] and the
//! on-disk directory in sync.
//!
//! This mirrors `tests/statuses.rs`; here each payload is a single group object.

use truthsocial::model::Group;

mod common;

/// The fixtures directory on disk, used only to detect drift from [`FIXTURES`].
const GROUPS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/groups/by-id");

/// Expands to a `(name, json)` fixture entry, embedding `data/groups/by-id/<name>` at compile time
/// so the filename is written exactly once.
macro_rules! fixture {
    ($name:literal) => {
        ($name, include_str!(concat!("data/groups/by-id/", $name)))
    };
}

/// Every single-group fixture, keyed by its `<group-id>.json` filename. A new fixture must be
/// registered here; [`group_fixtures_directory_matches_enumeration`] fails on any divergence.
const FIXTURES: &[(&str, &str)] = &[
    fixture!("110397125758772642.json"),
    fixture!("111069914752282213.json"),
];

#[test]
fn group_fixtures_parse_as_group() {
    for &(name, json) in FIXTURES {
        let group = serde_json::from_str::<Group<'_>>(json)
            .unwrap_or_else(|error| panic!("{name} failed to parse as Group: {error}"));

        // The single-group view carries the member-avatar preview; assert it is captured rather
        // than silently dropped, guarding the `member_avatars` wiring.
        assert!(
            group
                .member_avatars
                .is_some_and(|avatars| !avatars.is_empty()),
            "{name} parsed with no member_avatars"
        );
    }
}

#[test]
fn group_fixtures_directory_matches_enumeration() {
    common::assert_directory_matches(GROUPS_DIR, FIXTURES);
}
