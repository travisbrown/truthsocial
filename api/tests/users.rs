//! Schema-coverage guard for the curated account API-response fixtures.
//!
//! Every file in `tests/data/users/` is a real Truth Social account API response, named
//! `<account-id>.json`. Fixtures are embedded at compile time via [`FIXTURES`] and asserted
//! to deserialize as [`Account`], catching any model regression that stops covering them. A
//! second test keeps [`FIXTURES`] and the on-disk directory in sync.
//!
//! This mirrors `tests/statuses.rs`; each file is a single account object.

use truthsocial::model::Account;

mod common;

/// The fixtures directory on disk, used only to detect drift from [`FIXTURES`].
const USERS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/users");

/// Expands to a `(name, json)` fixture entry, embedding `data/users/<name>` at compile time so
/// the filename is written exactly once. `include_str!` resolves the `concat!`-built path
/// relative to this source file.
macro_rules! fixture {
    ($name:literal) => {
        ($name, include_str!(concat!("data/users/", $name)))
    };
}

/// Every account fixture, keyed by its `<account-id>.json` filename. A new fixture must be
/// registered here; [`user_fixtures_directory_matches_enumeration`] fails if the directory and
/// this list diverge.
const FIXTURES: &[(&str, &str)] = &[
    fixture!("107764331655353190.json"),
    fixture!("107764331655353191.json"),
    fixture!("107771700550828156.json"),
    fixture!("107777378958166346.json"),
    fixture!("107780257626128497.json"),
    fixture!("107803725277820351.json"),
    fixture!("107804720352471774.json"),
    fixture!("107808142265513695.json"),
    fixture!("107808859557707601.json"),
    fixture!("107810096875431988.json"),
    fixture!("107814273983601303.json"),
    fixture!("107820765822139727.json"),
    fixture!("107821507867121730.json"),
    fixture!("107821716695904267.json"),
    fixture!("107825028741368560.json"),
    fixture!("107826559359582068.json"),
    fixture!("107833648982461195.json"),
    fixture!("107834149647767059.json"),
    fixture!("107834318527894167.json"),
    fixture!("107834323082119575.json"),
    fixture!("107834540868570927.json"),
    fixture!("107834585951832811.json"),
    fixture!("107834674875471949.json"),
    fixture!("107834820349332678.json"),
    fixture!("107835285473427854.json"),
    fixture!("107837183104785660.json"),
    fixture!("107837596294318659.json"),
    fixture!("107839464150689525.json"),
    fixture!("107841586977713410.json"),
    fixture!("107842372228530221.json"),
    fixture!("108274591401172033.json"),
    fixture!("108318222345080429.json"),
    fixture!("108318634282795193.json"),
    fixture!("108353788151996932.json"),
    fixture!("115294575350240244.json"),
];

#[test]
fn user_fixtures_parse_as_account() {
    for &(name, json) in FIXTURES {
        // Borrowed deserialization, matching the API client's code path; the parsed value
        // borrows from `json`, which is `'static` here.
        if let Err(error) = serde_json::from_str::<Account<'_>>(json) {
            panic!("{name} failed to parse as Account: {error}");
        }
    }
}

#[test]
fn user_fixtures_directory_matches_enumeration() {
    common::assert_directory_matches(USERS_DIR, FIXTURES);
}
