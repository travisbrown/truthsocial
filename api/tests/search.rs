//! Schema-coverage guard for the curated `GET /api/v2/search` response fixtures.
//!
//! Each file under `tests/data/search/<facet>/` is a real search response for that facet, named
//! `<query>.json`: `groups/` holds `type=groups` responses and `topics/` holds `type=topics`
//! (hashtag) responses. Fixtures are embedded at compile time and asserted to deserialize as
//! [`SearchResults`], exercising the fields that appear only in search results: the group
//! [`position`](truthsocial::model::Group::position) and the hashtag
//! [`history`](truthsocial::model::Tag::history) / `recent_*` clusters. A per-facet test keeps the
//! embedded lists and the on-disk directories in sync.
//!
//! This mirrors `tests/statuses.rs`; here each payload is a composite `SearchResults` object
//! rather than a single entity.

use truthsocial_api::types::SearchResults;

mod common;

/// Expands to a `(name, json)` fixture entry, embedding `data/search/<dir>/<name>` at compile time
/// so each filename is written exactly once.
macro_rules! fixture {
    ($dir:literal, $name:literal) => {
        (
            $name,
            include_str!(concat!("data/search/", $dir, "/", $name)),
        )
    };
}

/// The `type=groups` fixtures, keyed by `<query>.json` filename.
const GROUP_FIXTURES: &[(&str, &str)] = &[
    fixture!("groups", "hitler.json"),
    fixture!("groups", "trump.json"),
];

/// The `type=topics` (hashtag) fixtures, keyed by `<query>.json` filename.
const TOPIC_FIXTURES: &[(&str, &str)] = &[fixture!("topics", "whitelivesmatter.json")];

/// Deserialize every fixture in `set`, panicking with the filename on failure.
fn parse_all(set: &[(&str, &str)]) -> Vec<SearchResults> {
    set.iter()
        .map(|&(name, json)| {
            serde_json::from_str::<SearchResults>(json)
                .unwrap_or_else(|error| panic!("{name} failed to parse as SearchResults: {error}"))
        })
        .collect()
}

/// Assert that the fixtures embedded in `set` exactly match the files in `tests/data/search/<facet>`.
fn assert_facet_matches(facet: &str, set: &[(&str, &str)]) {
    let path = format!("{}/tests/data/search/{facet}", env!("CARGO_MANIFEST_DIR"));
    common::assert_directory_matches(&path, set);
}

#[test]
fn group_search_fixtures_populate_groups_and_position() {
    let results = parse_all(GROUP_FIXTURES);

    // A `type=groups` search fills the `groups` facet.
    assert!(
        results.iter().all(|r| !r.groups.is_empty()),
        "a group fixture parsed with no groups"
    );

    // `position` is sparse (some result sets omit it entirely), so it is guarded once in aggregate:
    // this catches a silent regression of the search-only field to `None` everywhere.
    assert!(
        results
            .iter()
            .flat_map(|r| &r.groups)
            .any(|group| group.position.is_some()),
        "no group across the fixtures carried a search `position`"
    );
}

#[test]
fn topic_search_fixtures_populate_hashtag_history() {
    let results = parse_all(TOPIC_FIXTURES);

    // A `type=topics` search fills the `hashtags` facet.
    assert!(
        results.iter().all(|r| !r.hashtags.is_empty()),
        "a topic fixture parsed with no hashtags"
    );

    // Guards the search-only hashtag fields against a silent regression to `None`; a populated
    // `history` also confirms the `integer_str`-decoded `TagHistory` counts round-trip.
    assert!(
        results.iter().flat_map(|r| &r.hashtags).any(|tag| {
            tag.history.as_ref().is_some_and(|h| !h.is_empty())
                && tag.recent_history.is_some()
                && tag.recent_statuses_count.is_some()
        }),
        "no hashtag across the fixtures carried search `history`/`recent_*` fields"
    );
}

#[test]
fn search_fixture_directories_match_enumeration() {
    assert_facet_matches("groups", GROUP_FIXTURES);
    assert_facet_matches("topics", TOPIC_FIXTURES);
}
