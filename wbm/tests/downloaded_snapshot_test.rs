//! Parses a real Wayback Machine snapshot that is downloaded on demand rather than checked in.
//!
//! The `J3O6...` capture is a gzip-compressed account-statuses page whose payload exercises two
//! schema edge cases the committed fixtures do not: a status with `visibility: null` and a media
//! attachment reporting `processing: "queued"`. It is fetched through
//! [`archivindex_wbm_test_data::Cache`] into a gitignored cache, so the ~20 KB archive never
//! enters the repository. When the archive is unavailable (no network, or the Wayback Machine
//! declines to serve it) the cache yields `None` and the test skips instead of failing.
//!
//! The archive is handled through the archivindex gzip codec, exactly as ingest does: its
//! parameters are inferred, its content is decoded, and re-encoding under those parameters must
//! reproduce the archive byte-for-byte (so the reproduced bytes hash back to the digest that named
//! the file). Decoding alone with `flate2` is deliberately avoided: `flate2::read::GzDecoder`
//! decompresses captures the codec cannot reproduce, which would let an unverifiable snapshot pass
//! as valid.

use archivindex_wbm::digest::Sha1Digest;
use archivindex_wbm_json_gzip::{GzipParams, codec};
use archivindex_wbm_test_data::Cache;
use truthsocial::model::StatusContent;

/// The original (`id_`) rendering of an account's statuses page; only its raw bytes hash to
/// `DIGEST`, which is why the cache requests that rendering.
const URL: &str = "https://truthsocial.com/api/v1/accounts/107834825870339843/statuses?exclude_replies=true&with_muted=true";
const TIMESTAMP: &str = "20221212003808";
const DIGEST: &str = "J3O6LXGYKPM2YA6S2W7FNRDAYBAM6BFB";
const CACHE_DIRECTORY: &str = "tests/data/.cache";

#[tokio::test]
async fn parses_downloaded_account_statuses_snapshot() {
    let cache = Cache::new(CACHE_DIRECTORY).expect("Cannot build HTTP client");

    let Some(archive) = cache
        .bytes(
            URL,
            TIMESTAMP.parse().expect("Invalid test timestamp"),
            DIGEST.parse().expect("Invalid test digest"),
        )
        .await
        .expect("Unexpected I/O error")
    else {
        // The snapshot is unavailable (offline, or the archive declined it): treat it as a skip.
        return;
    };

    let codec = codec();

    // Infer the gzip parameters (as ingest does) so the archive can be reproduced and verified.
    let params = GzipParams::infer(&archive)
        .expect("the archivindex gzip codec must infer this snapshot's parameters");
    let content = codec
        .decode(&archive)
        .expect("the downloaded snapshot must decode to text");

    // Re-encoding under the inferred parameters must reproduce the archive byte-for-byte, so the
    // reproduced bytes still hash to the digest that named the file.
    let reproduced = codec.encode(content.as_ref(), &params.format_info().metadata);
    assert_eq!(
        reproduced.as_ref(),
        archive.as_ref(),
        "byte-exact gzip round-trip failed for {DIGEST}",
    );
    assert_eq!(
        Sha1Digest::compute(reproduced.as_ref()).to_string(),
        DIGEST,
        "reproduced bytes for {DIGEST} do not hash back to its name",
    );

    // The verified content must deserialize against the core `StatusContent` schema. This capture
    // is a statuses page, so its content is the array form.
    let StatusContent::Multiple(statuses) =
        serde_json::from_str(content.as_ref()).expect("the snapshot must parse as StatusContent")
    else {
        panic!("expected an array of statuses");
    };
    assert!(
        !statuses.is_empty(),
        "the account-statuses page should contain at least one status",
    );
}
