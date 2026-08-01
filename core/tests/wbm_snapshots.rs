//! Schema-coverage guard for the curated Wayback Machine snapshot fixtures.
//!
//! Every file in `tests/data/wbm/snapshots/` is a real Truth Social API payload,
//! named by the uppercase Base32 SHA-1 digest of its original bytes. Those bytes are
//! stored exactly as the origin served them, so some are gzip-compressed (magic
//! `1f 8b`). Fixtures are embedded at compile time via [`FIXTURES`] and asserted to
//! deserialize as [`StatusContent`], catching any model regression that stops covering
//! them. A second test keeps [`FIXTURES`] and the on-disk directory in exact sync.

use flate2::read::GzDecoder;
use std::collections::BTreeSet;
use std::io::Read;
use truthsocial::model::StatusContent;

/// The snapshot directory on disk, used only to detect drift from [`FIXTURES`].
const SNAPSHOTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/wbm/snapshots");

/// Expands to a `(name, bytes)` fixture entry, embedding `data/wbm/snapshots/<name>`
/// at compile time so the digest filename is written exactly once. `include_bytes!`
/// resolves the `concat!`-built path relative to this source file.
macro_rules! fixture {
    ($name:literal) => {
        ($name, include_bytes!(concat!("data/wbm/snapshots/", $name)))
    };
}

/// Every snapshot fixture, keyed by its Base32 SHA-1 digest filename. A new fixture
/// must be registered here; [`wbm_snapshots_directory_matches_enumeration`] fails if
/// the directory and this list diverge.
const FIXTURES: &[(&str, &[u8])] = &[
    fixture!("5QIKZOW6KFPXX3Q2UNR4HORDC2IPHFSZ"),
    fixture!("5QSJEP7ZFGQOYB67GRSXLDOUWUIRXU5F"),
    fixture!("66PZYNINJYRI2POZZWAXIIFIXF6LB5N4"),
    fixture!("AAA7WWHWJB6AP4HLEMQBVJQP6QSDQV4Q"),
    fixture!("AAACL3SP57UDUZWUNKYLBS5GRVVO57JF"),
    fixture!("AAAJOS3WOYRYLASLVLDYFDU7EE44SHR2"),
    fixture!("AAALGMEID4SNBKZ2RUBCKBGWTEFYDWNC"),
    fixture!("AAAYPJ7A6NDOBW3KXH57ZZKVQ6L4WD6M"),
    fixture!("AABIATA5YXC7LCEPOU24U5FN6ULMILL2"),
    fixture!("AABTQ7X6P2QZM6NMUF244XZ72YJXRYUC"),
    fixture!("AAEGIH23RWGRA5POU7F2CEOLYWYM664F"),
    fixture!("AAFRHNICTL6NECSFNHT4U3QTCRTEFIUV"),
    fixture!("AAGCDRPMZ65KGPCTAGSZZMJCDHKKR3HA"),
    fixture!("AAINYHXV73DIWMOPHE7372JKHHSPJBAV"),
    fixture!("AAIXV7K55SAKNDTB2CLY4HLD62IHHL45"),
    fixture!("AANYHQREUL55MLX5HNJ2LEEZZXXL7NDY"),
    fixture!("AAPMRTEIFA73WIPVUAVCX6TWZHFF3VJX"),
    fixture!("AARDSO2IWOC37RSCMF66KUVLXHOS2OI6"),
    fixture!("AASQSDZXOZ2C7Z2UMTZTIJ63I33KY6A3"),
    fixture!("ABAY3PZXPQGFWHFIWPCQLDPPK7YLVM6B"),
    fixture!("ACEWLCRAJMERJRZS4UMDE36SPTSJ2UVP"),
    fixture!("ACIOAULA7PEKDUZPYMJHCW3OSVWW6OKY"),
    fixture!("ACVMUEPKYLNRAHN3XCYPFLER2WENJR5K"),
    fixture!("ADDTCWFH37EYIWPVHC7LUXQ6NND5YAZM"),
    fixture!("ADDYAKP3DF4Z4B7UCYI55KXQ2MMOLLAP"),
    fixture!("AFOLBGJSXEG7R5OK2IFC24A7NDRITY7E"),
    fixture!("AJ5CMWDL6TXYHJOJ4E56WMOXDJSTPNJQ"),
    fixture!("AKMGEYMCSVQH4DJDHDYDASUO2X3Y7K5M"),
    fixture!("AQG4XARXYVOUXENARZSJE6BFP7DI4KL7"),
    fixture!("AQJSKNMCZ7BV6QIFP4CBKATBQ5GSHYLC"),
    fixture!("BIA72WO7SXZQREC6CVQZRBLAVENQXK5A"),
    fixture!("BNS7Q4IX22KYSM35D5AQXOUD4XUOPVQV"),
    fixture!("CMGMD7EO5E4JGZSUOIL2BI22QS5ZYO5R"),
    fixture!("CWYULNN2U3S3N53WOTE2UR4HXJVBOGUD"),
    fixture!("LR2C2SGUH5OHC7IZCTO7R54W3MQTHOKO"),
    fixture!("QQL6T4SOGYGLPQ6DJ6K7QE7GD7MYUYYH"),
    fixture!("SCVL56KVDXXYKYZN77QYTSHUVDEOG2PV"),
    fixture!("U7KWUID6CSUSPK3RRLX3QUUMSNBJS3AT"),
    fixture!("ZFLYFOFEVCV6ULVCSCVGTRVSLLDETJK4"),
];

/// Decode a raw snapshot payload to its UTF-8 JSON text, transparently
/// gunzipping when the gzip magic bytes are present.
fn decode(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut text = String::new();
        GzDecoder::new(bytes)
            .read_to_string(&mut text)
            .expect("gzip payload decompresses to text");
        text
    } else {
        String::from_utf8(bytes.to_vec()).expect("payload is UTF-8 JSON")
    }
}

#[test]
fn wbm_snapshots_parse_as_status_content() {
    for &(name, bytes) in FIXTURES {
        let json = decode(bytes);
        // Borrowed deserialization, matching the `read-compact` code path; the parsed
        // value borrows from `json`, which outlives it within this iteration.
        if let Err(error) = serde_json::from_str::<StatusContent<'_>>(&json) {
            panic!("{name} failed to parse as StatusContent: {error}");
        }
    }
}

#[test]
fn wbm_snapshots_directory_matches_enumeration() {
    let enumerated = FIXTURES
        .iter()
        .map(|&(name, _)| name)
        .collect::<BTreeSet<_>>();
    let on_disk = std::fs::read_dir(SNAPSHOTS_DIR)
        .expect("snapshots directory exists")
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
        "files present in {SNAPSHOTS_DIR} but not registered in FIXTURES: {unregistered:?}"
    );
    let missing = enumerated.difference(&on_disk).collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "fixtures registered in FIXTURES but missing from disk: {missing:?}"
    );
}
