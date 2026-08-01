//! Classification of Truth Social API URLs.

/// A URL does not identify a supported status endpoint.
#[derive(Debug, thiserror::Error)]
pub enum StatusUrlError {
    /// The URL is malformed or does not identify an individual status.
    #[error("not a Truth Social status URL")]
    InvalidUrl,
    /// The complete numeric status ID does not fit in a `u64`.
    #[error("status ID is not a valid u64")]
    InvalidId(#[source] std::num::ParseIntError),
}

/// Extract the ID of an individual Truth Social API status.
///
/// Accepts HTTPS URLs on `truthsocial.com` at `/api/v1/statuses/{id}`, with a nonempty
/// ASCII-decimal ID. Queries and fragments are ignored. URL parsing normalizes host casing and the
/// default HTTPS port; credentials, nondefault ports, trailing slashes, and subresources such as
/// `/context` are rejected.
pub fn parse_status_id(value: &str) -> Result<u64, StatusUrlError> {
    let url = url::Url::parse(value).map_err(|_| StatusUrlError::InvalidUrl)?;
    if url.scheme() != "https"
        || url.host_str() != Some("truthsocial.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(StatusUrlError::InvalidUrl);
    }
    let id = url
        .path()
        .strip_prefix("/api/v1/statuses/")
        .filter(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or(StatusUrlError::InvalidUrl)?;
    id.parse().map_err(StatusUrlError::InvalidId)
}

#[cfg(test)]
mod tests {
    use super::{StatusUrlError, parse_status_id};

    #[test]
    fn accepts_status_endpoints_and_ignores_query_and_fragment() {
        for url in [
            "https://truthsocial.com/api/v1/statuses/123",
            "https://truthsocial.com/api/v1/statuses/123?foo=bar",
            "https://truthsocial.com/api/v1/statuses/123#fragment",
            "https://TRUTHSOCIAL.COM:443/api/v1/statuses/123?foo=bar#fragment",
        ] {
            assert_eq!(parse_status_id(url).unwrap(), 123, "{url}");
        }
        assert_eq!(
            parse_status_id("https://truthsocial.com/api/v1/statuses/18446744073709551615")
                .unwrap(),
            u64::MAX,
        );
    }

    #[test]
    fn rejects_other_origins_and_non_status_paths() {
        for url in [
            "https://truthsocialXcom/api/v1/statuses/123",
            "https://truthsocial.com.example.org/api/v1/statuses/123",
            "https://example.org/?next=https://truthsocial.com/api/v1/statuses/123",
            "http://truthsocial.com/api/v1/statuses/123",
            "https://truthsocial.com:444/api/v1/statuses/123",
            "https://user:password@truthsocial.com/api/v1/statuses/123",
            "https://truthsocial.com@evil.org/api/v1/statuses/123",
            "https://truthsocial.com/api/v1/statuses/123abc",
            "https://truthsocial.com/api/v1/statuses/123/context",
            "https://truthsocial.com/api/v1/statuses/123/",
            "https://truthsocial.com/api/v1/statuses/",
            "https://truthsocial.com/api/v1/statuses/+123",
            "https://truthsocial.com/api/v1/statuses/１２３",
            "https://truthsocial.com/api/v1/statuses/%31%32%33",
            "https://truthsocial.com/api/v1/accounts/123",
            "https://truthsocial.com/@username/123",
            "https://truthsocial.com/users/username/statuses/123",
            "https://truthsocial.com/other/api/v1/statuses/123",
            "not a url",
        ] {
            assert!(
                matches!(parse_status_id(url), Err(StatusUrlError::InvalidUrl)),
                "{url}"
            );
        }
    }

    #[test]
    fn distinguishes_overflow_from_a_non_status_url() {
        assert!(matches!(
            parse_status_id("https://truthsocial.com/api/v1/statuses/18446744073709551616"),
            Err(StatusUrlError::InvalidId(_)),
        ));
    }
}
