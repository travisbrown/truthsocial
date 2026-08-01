//! Parses real Wayback Machine snapshots that are downloaded on demand rather than checked in.
//!
//! Each capture is fetched through [`archivindex_wbm_test_data::Cache`] into a gitignored cache, so
//! the archives never enter the repository. When one is unavailable (no network, or the Wayback
//! Machine declines to serve it) the cache yields `None` and the test skips instead of failing.
//!
//! Each capture must match the SHA-1 digest that names it and deserialize as the expected response
//! type. A gzip capture is additionally handled through the archivindex gzip codec exactly as
//! ingest does: its parameters are inferred, its content is decoded, and re-encoding under those
//! parameters must reproduce the archive byte-for-byte. Decoding a gzip capture with `flate2` alone
//! is deliberately avoided: `flate2::read::GzDecoder` decompresses captures the codec cannot
//! reproduce, which would let an unverifiable snapshot pass as valid.

use archivindex_wbm::digest::Sha1Digest;
use archivindex_wbm_json_gzip::{GzipParams, codec};
use archivindex_wbm_test_data::Cache;
use truthsocial::model::{ResponseBody, StatusContent};

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

/// An account endpoint capture: a single account object (not a statuses page), stored as plain
/// UTF-8 response bytes rather than a gzip archive.
const ACCOUNT_URL: &str = "https://truthsocial.com/api/v1/accounts/112379266475954238";
const ACCOUNT_TIMESTAMP: &str = "20260102183502";
const ACCOUNT_DIGEST: &str = "3GBVF25NJQGFFS76UPQKM7GEKE2EVORR";

/// Downloads a snapshot and verifies it against the digest that names it, returning its decoded
/// JSON text. Returns `None` (a skip) when the snapshot is unavailable.
///
/// The archived bytes must hash back to `digest`. A gzip archive is additionally round-tripped
/// through the codec: inferring its parameters guarantees a byte-exact reproduction, so re-encoding
/// must yield the exact archive bytes. A plain archive is decoded as its UTF-8 bytes.
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

    // The archive is the digest-named byte stream, so it must hash back to that name. The cache
    // already checks this; asserting here keeps the guarantee visible in the test.
    assert_eq!(
        Sha1Digest::compute(archive.as_ref()).to_string(),
        digest,
        "archived bytes for {digest} do not hash back to its name",
    );

    let content = if archive.starts_with(&[0x1f, 0x8b]) {
        let codec = codec();
        let params = GzipParams::infer(&archive).unwrap_or_else(|| {
            panic!("the archivindex gzip codec must infer {digest}'s parameters")
        });
        let decoded = codec
            .decode(&archive)
            .unwrap_or_else(|| panic!("{digest} must decode to text"));

        let reproduced = codec.encode(decoded.as_ref(), &params.format_info().metadata);
        assert_eq!(
            reproduced.as_ref(),
            archive.as_ref(),
            "byte-exact gzip round-trip failed for {digest}",
        );
        decoded.into_owned()
    } else {
        String::from_utf8(archive.to_vec()).unwrap_or_else(|_| panic!("{digest} is not UTF-8 text"))
    };

    Some(content)
}

/// Asserts that `content` (a verified capture) deserializes as a non-empty array of statuses.
fn assert_parses_as_statuses_page(content: &str, digest: &str) {
    let ResponseBody::Statuses(StatusContent::Multiple(statuses)) =
        serde_json::from_str(content).expect("the snapshot must parse as a response body")
    else {
        panic!("{digest}: expected an array of statuses");
    };
    assert!(
        !statuses.is_empty(),
        "{digest}: the account-statuses page should contain at least one status",
    );
}

/// Asserts that `content` (a verified capture) deserializes as a single account object.
fn assert_parses_as_account(content: &str, digest: &str) {
    let body = serde_json::from_str::<ResponseBody<'_>>(content)
        .expect("the snapshot must parse as a response body");
    assert!(
        matches!(body, ResponseBody::Account(_)),
        "{digest}: expected an account object",
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

#[tokio::test]
async fn parses_account_snapshot() {
    let Some(content) =
        fetch_verified_content(ACCOUNT_URL, ACCOUNT_TIMESTAMP, ACCOUNT_DIGEST).await
    else {
        return;
    };
    assert_parses_as_account(&content, ACCOUNT_DIGEST);
}
