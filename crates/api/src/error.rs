//! The error type returned by client operations.

/// An error produced by a [`Client`](crate::client::Client) request.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The HTTP transport failed (DNS resolution, TLS, connection, timeout, …).
    #[error("HTTP transport error")]
    Transport(#[from] wreq::Error),

    /// The API responded with a non-success status code. The body is retained for diagnosis (it is
    /// often a JSON `{"error": "..."}` payload, or a Cloudflare challenge page).
    #[error("API returned HTTP {status}: {}", .body.escape_default())]
    Api {
        /// The HTTP status code.
        status: wreq::StatusCode,
        /// The response body.
        body: String,
    },

    /// The OAuth password-grant login failed.
    #[error("authentication failed: {0}")]
    Auth(String),

    /// A request that requires a bearer token was issued by an unauthenticated client.
    #[error("this request requires authentication; call `Client::login` or build with a token")]
    Unauthenticated,

    /// A response body could not be deserialized into the expected Rust type.
    #[error("failed to deserialize the response as `{target}`")]
    Decode {
        /// The Rust type the body was being parsed into.
        target: &'static str,
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// A request URL could not be constructed from the configured base URL.
    #[error("invalid URL")]
    Url(#[from] url::ParseError),
}

/// A convenient result alias for client operations.
pub type Result<T> = std::result::Result<T, Error>;
