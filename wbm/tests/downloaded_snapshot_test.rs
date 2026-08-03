//! Parses real Wayback Machine snapshots that are downloaded on demand rather than checked in.
//!
//! Each capture is fetched through [`archivindex_wbm_test_data::Cache`] into a gitignored cache, so
//! the archives never enter the repository. When one is unavailable (no network, or the Wayback
//! Machine declines to serve it) the cache yields `None` and the test skips instead of failing.
//!
//! Every archive is handled through the archivindex gzip codec, exactly as ingest does: its
//! parameters are inferred, its content is decoded, and re-encoding under those parameters must
//! reproduce the archive byte-for-byte (so the reproduced bytes hash back to the digest that named
//! the file). Decoding alone with `flate2` is deliberately avoided: `flate2::read::GzDecoder`
//! decompresses captures the codec cannot reproduce, which would let an unverifiable snapshot pass
//! as valid.

use archivindex_wbm::digest::Sha1Digest;
use archivindex_wbm_json_gzip::{GzipParams, codec};
use archivindex_wbm_test_data::Cache;
use truthsocial::model::StatusContent;

/// The gitignored directory each snapshot is downloaded into.
const CACHE_DIRECTORY: &str = "tests/data/.cache";

/// A statuses page whose gzip archive needs the codec's flush-offset tracking to reproduce, and
/// whose payload carries a status with `visibility: null` and a media attachment reporting
/// `processing: "queued"`.
const NULL_VISIBILITY_URL: &str = "https://truthsocial.com/api/v1/accounts/107834825870339843/statuses?exclude_replies=true&with_muted=true";
const NULL_VISIBILITY_TIMESTAMP: &str = "20221212003808";
const NULL_VISIBILITY_DIGEST: &str = "J3O6LXGYKPM2YA6S2W7FNRDAYBAM6BFB";

/// A statuses page carrying a media attachment of type `unknown` with a null `meta`.
const UNKNOWN_MEDIA_URL: &str = "https://truthsocial.com/api/v1/accounts/115829601258051869/statuses?exclude_replies=true&only_replies=false&with_muted=true";
const UNKNOWN_MEDIA_TIMESTAMP: &str = "20260729013257";
const UNKNOWN_MEDIA_DIGEST: &str = "L7JN3IVKTLW77BS2Q5DVZNRAWJRYTIZ6";

/// Downloads a snapshot and verifies it through the archivindex gzip codec, returning its decoded
/// JSON text. Returns `None` (a skip) when the snapshot is unavailable.
///
/// Inferring the parameters guarantees the archive reproduces byte-for-byte, so re-encoding must
/// yield the exact archive bytes, which must in turn hash back to `digest`.
async fn fetch_verified_content(url: &str, timestamp: &str, digest: &str) -> Option<String> {
    let cache = Cache::new(CACHE_DIRECTORY).expect("Cannot build HTTP client");
    let archive = cache
        .bytes(
            url,
            timestamp.parse().expect("Invalid test timestamp"),
            digest.parse().expect("Invalid test digest"),
        )
        .await
        .expect("Unexpected I/O error")?;

    let codec = codec();
    let params = GzipParams::infer(&archive)
        .unwrap_or_else(|| panic!("the archivindex gzip codec must infer {digest}'s parameters"));
    let content = codec
        .decode(&archive)
        .unwrap_or_else(|| panic!("{digest} must decode to text"));

    let reproduced = codec.encode(content.as_ref(), &params.format_info().metadata);
    assert_eq!(
        reproduced.as_ref(),
        archive.as_ref(),
        "byte-exact gzip round-trip failed for {digest}",
    );
    assert_eq!(
        Sha1Digest::compute(reproduced.as_ref()).to_string(),
        digest,
        "reproduced bytes for {digest} do not hash back to its name",
    );

    Some(content.into_owned())
}

/// Asserts that `content` (a verified statuses page) deserializes as a non-empty array of statuses.
fn assert_parses_as_statuses_page(content: &str, digest: &str) {
    let StatusContent::Multiple(statuses) =
        serde_json::from_str(content).expect("the snapshot must parse as StatusContent")
    else {
        panic!("{digest}: expected an array of statuses");
    };
    assert!(
        !statuses.is_empty(),
        "{digest}: the account-statuses page should contain at least one status",
    );
}

#[tokio::test]
async fn parses_null_visibility_snapshot() {
    let Some(content) = fetch_verified_content(
        NULL_VISIBILITY_URL,
        NULL_VISIBILITY_TIMESTAMP,
        NULL_VISIBILITY_DIGEST,
    )
    .await
    else {
        return;
    };
    assert_parses_as_statuses_page(&content, NULL_VISIBILITY_DIGEST);
}

#[tokio::test]
async fn parses_unknown_media_snapshot() {
    let Some(content) = fetch_verified_content(
        UNKNOWN_MEDIA_URL,
        UNKNOWN_MEDIA_TIMESTAMP,
        UNKNOWN_MEDIA_DIGEST,
    )
    .await
    else {
        return;
    };
    assert_parses_as_statuses_page(&content, UNKNOWN_MEDIA_DIGEST);
}
