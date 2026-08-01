//! Schema-coverage guard for the curated status API-response fixtures.
//!
//! Every file in `tests/data/statuses/` is a real Truth Social status API response,
//! named `<status-id>.json`. Fixtures are embedded at compile time via [`FIXTURES`] and
//! asserted to deserialize as [`Status`], catching any model regression that stops
//! covering them. A second test keeps [`FIXTURES`] and the on-disk directory in sync.
//!
//! This mirrors `core/tests/wbm_snapshots.rs`; here the payloads are always plain UTF-8
//! JSON (never gzip), and each file is a single status object rather than a
//! `StatusContent` that may also be an array.

use truthsocial::model::Status;

mod common;

/// The fixtures directory on disk, used only to detect drift from [`FIXTURES`].
const STATUSES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/statuses");

/// Expands to a `(name, json)` fixture entry, embedding `data/statuses/<name>` at
/// compile time so the filename is written exactly once. `include_str!` resolves the
/// `concat!`-built path relative to this source file.
macro_rules! fixture {
    ($name:literal) => {
        ($name, include_str!(concat!("data/statuses/", $name)))
    };
}

/// Every status fixture, keyed by its `<status-id>.json` filename. A new fixture must be
/// registered here; [`status_fixtures_directory_matches_enumeration`] fails if the
/// directory and this list diverge.
const FIXTURES: &[(&str, &str)] = &[
    fixture!("107814993046697416.json"),
    fixture!("107820778259039972.json"),
    fixture!("107820803891719077.json"),
    fixture!("107836154544993176.json"),
    fixture!("107838474127241768.json"),
    fixture!("107957941707714512.json"),
    fixture!("107962811324118694.json"),
    fixture!("108046466782901127.json"),
    fixture!("108407676531874529.json"),
    fixture!("108449612593939663.json"),
    fixture!("108521529318484904.json"),
    fixture!("108742189401812078.json"),
    fixture!("109020896871857890.json"),
    fixture!("109366289608305434.json"),
    fixture!("109372091198783453.json"),
    fixture!("109864160311151506.json"),
    fixture!("109867113754520597.json"),
    fixture!("109901674260251212.json"),
    fixture!("109901708294496396.json"),
    fixture!("109967811792793526.json"),
    fixture!("110049307881642835.json"),
    fixture!("110374797872162426.json"),
    fixture!("110375104524907659.json"),
    fixture!("110549629953246687.json"),
    fixture!("110549706890738623.json"),
    fixture!("110617267048515904.json"),
    fixture!("110651508324557686.json"),
    fixture!("110941119092973173.json"),
    fixture!("114433942825461076.json"),
    fixture!("115396458435977605.json"),
    fixture!("115396468279137711.json"),
    fixture!("115396476953854230.json"),
    fixture!("115511662605004445.json"),
    fixture!("115622124123085049.json"),
    fixture!("115640260388917744.json"),
    fixture!("115667454848316967.json"),
    fixture!("115737662783453518.json"),
    fixture!("115806398111692077.json"),
    fixture!("115816897168008854.json"),
    fixture!("115864791948135518.json"),
    fixture!("116491559257751897.json"),
    fixture!("116521598497096244.json"),
    fixture!("116616050796377967.json"),
    fixture!("116654671527972217.json"),
    fixture!("116669584003805199.json"),
    fixture!("116684113664648014.json"),
    fixture!("116692873830981873.json"),
    fixture!("116711704834455679.json"),
    fixture!("116712058361368233.json"),
];

#[test]
fn status_fixtures_parse_as_status() {
    for &(name, json) in FIXTURES {
        // Borrowed deserialization, matching the API client's code path; the parsed
        // value borrows from `json`, which is `'static` here.
        if let Err(error) = serde_json::from_str::<Status<'_>>(json) {
            panic!("{name} failed to parse as Status: {error}");
        }
    }
}

#[test]
fn status_fixtures_directory_matches_enumeration() {
    common::assert_directory_matches(STATUSES_DIR, FIXTURES);
}
