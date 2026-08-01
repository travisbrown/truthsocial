//! The HTTP client and its endpoint methods.

use std::time::Duration;

use truthsocial::model::{Account, Status};
use url::Url;
use wreq_util::Emulation;

use crate::auth::{Credentials, TokenResponse};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::types::{Context, SearchKind, SearchResults, TimelineParams};

/// The default API origin.
pub const DEFAULT_BASE_URL: &str = "https://truthsocial.com";

/// Environment variable holding a Cloudflare `cf_clearance` cookie (see
/// [`ClientBuilder::cf_clearance`]).
pub const CF_CLEARANCE_ENV: &str = "CF_CLEARANCE";

/// The browser whose TLS/HTTP2 fingerprint and headers the client emulates by default.
///
/// Truth Social is fronted by Cloudflare, which fingerprints the TLS handshake (JA3/JA4) and HTTP/2
/// settings and rejects stock HTTP clients, even with a valid bearer token, with an HTTP 403
/// challenge. Emulating Firefox keeps the fingerprint coherent with [`DEFAULT_USER_AGENT`] (a
/// `cf_clearance` cookie is bound to the User-Agent, so the two must agree).
const DEFAULT_EMULATION: Emulation = Emulation::Firefox139;

/// The default `User-Agent`, matching a current Linux Firefox so it agrees with a `cf_clearance`
/// cookie obtained from that browser.
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:151.0) Gecko/20100101 Firefox/151.0";

/// An asynchronous Truth Social API client.
///
/// Construct one with [`Client::new`] (unauthenticated), [`Client::with_token`], or
/// [`Client::builder`]. Authenticate an existing client in place with [`Client::login`].
///
/// Requests are sent through a browser-impersonating HTTP client (see [`DEFAULT_EMULATION`]) so
/// they pass Cloudflare's bot checks.
///
/// The client is cheap to clone (the inner [`wreq::Client`] is reference-counted), but note that a
/// clone keeps its own copy of the bearer token; logging in on one clone does not affect others.
#[derive(Clone, Debug)]
pub struct Client {
    http: wreq::Client,
    base: Url,
    token: Option<String>,
    /// A pre-rendered `Cookie` header value (e.g. `cf_clearance=…`) sent on every request, or
    /// `None`.
    cookie_header: Option<String>,
}

impl Client {
    /// Create an unauthenticated client against the [default origin](DEFAULT_BASE_URL).
    ///
    /// # Errors
    ///
    /// Returns an error if the TLS backend or the default URL cannot be initialized.
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Create a client pre-loaded with a bearer token (e.g. one captured from a previous login).
    ///
    /// # Errors
    ///
    /// Returns an error if the client cannot be initialized.
    pub fn with_token(token: impl Into<String>) -> Result<Self> {
        Self::builder().token(token).build()
    }

    /// Start configuring a client (base URL, token, user agent, timeout).
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Whether the client currently holds a bearer token.
    #[must_use]
    pub const fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// The current bearer token, if any.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Set (or replace) the bearer token.
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    /// Exchange the given [`Credentials`] for a bearer token (OAuth password grant) and store it on
    /// the client.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`] if the server rejects the grant, or [`Error::Transport`] /
    /// [`Error::Decode`] for transport or response-shape failures.
    pub async fn login(&mut self, credentials: &Credentials) -> Result<()> {
        let url = self.base.join("/oauth/token")?;
        let response = self
            .prepare(self.http.post(url).form(&credentials.form_fields()))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(Error::Auth(format!("HTTP {status}: {body}")));
        }

        let token: TokenResponse = serde_json::from_str(&body).map_err(|source| Error::Decode {
            target: "oauth token response",
            source,
        })?;
        self.token = Some(token.access_token);
        Ok(())
    }

    // ── Typed endpoints ────────────────────────────────────────────────────────

    /// `GET /api/v1/accounts/verify_credentials`: the account the token belongs to.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unauthenticated`] if no token is set, otherwise any request error.
    pub async fn verify_credentials(&self) -> Result<Account<'static>> {
        if !self.is_authenticated() {
            return Err(Error::Unauthenticated);
        }
        self.get_json("/api/v1/accounts/verify_credentials", &[])
            .await
    }

    /// `GET /api/v1/accounts/lookup?acct=`: resolve a handle (without the leading `@`) to an
    /// account.
    ///
    /// # Errors
    ///
    /// Any request error (e.g. [`Error::Api`] with HTTP 404 if the handle does not exist).
    pub async fn lookup_account(&self, acct: &str) -> Result<Account<'static>> {
        self.get_json("/api/v1/accounts/lookup", &[("acct", acct.to_owned())])
            .await
    }

    /// `GET /api/v1/accounts/:id`: fetch an account by its numeric id.
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn account(&self, id: &str) -> Result<Account<'static>> {
        self.get_json(&format!("/api/v1/accounts/{id}"), &[]).await
    }

    /// `GET /api/v1/accounts/:id/statuses`: an account's statuses.
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn account_statuses(
        &self,
        id: &str,
        params: &TimelineParams,
    ) -> Result<Vec<Status<'static>>> {
        self.get_json(&format!("/api/v1/accounts/{id}/statuses"), &params.query())
            .await
    }

    /// `GET /api/v1/statuses/:id`: a single status.
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn status(&self, id: &str) -> Result<Status<'static>> {
        self.get_json(&format!("/api/v1/statuses/{id}"), &[]).await
    }

    /// `GET /api/v1/statuses/:id/context`: a status's ancestors and descendants.
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn status_context(&self, id: &str) -> Result<Context> {
        self.get_json(&format!("/api/v1/statuses/{id}/context"), &[])
            .await
    }

    /// `GET /api/v2/search?q=`: search accounts, statuses, and hashtags.
    ///
    /// Pass a [`SearchKind`] to restrict the search, or `None` to query all kinds. `resolve` asks
    /// the server to resolve a non-local handle or URL (it is ignored unless authenticated).
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn search(
        &self,
        query: &str,
        kind: Option<SearchKind>,
        resolve: bool,
    ) -> Result<SearchResults> {
        self.get_json("/api/v2/search", &Self::search_params(query, kind, resolve))
            .await
    }

    /// `GET /api/v2/search?q=`: like [`search`](Self::search) but returns the raw JSON body.
    ///
    /// The typed [`search`](Self::search) discards response fields this crate does not model (for
    /// example the `groups` facet); this variant preserves the payload verbatim, which is useful
    /// for capturing fixtures or reaching search facets not yet represented in [`SearchResults`].
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-success status, or [`Error::Transport`] / [`Error::Url`].
    pub async fn search_raw(
        &self,
        query: &str,
        kind: Option<SearchKind>,
        resolve: bool,
    ) -> Result<String> {
        self.get_text("/api/v2/search", &Self::search_params(query, kind, resolve))
            .await
    }

    /// Build the query-string parameters for `GET /api/v2/search`.
    fn search_params(
        query: &str,
        kind: Option<SearchKind>,
        resolve: bool,
    ) -> Vec<(&'static str, String)> {
        let mut params = vec![("q", query.to_owned()), ("resolve", resolve.to_string())];
        if let Some(kind) = kind {
            params.push(("type", kind.as_str().to_owned()));
        }
        params
    }

    /// `GET /api/v1/timelines/public`: the public (federated) timeline.
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn public_timeline(&self, params: &TimelineParams) -> Result<Vec<Status<'static>>> {
        self.get_json("/api/v1/timelines/public", &params.query())
            .await
    }

    /// `GET /api/v1/timelines/tag/:hashtag`: statuses for a hashtag (without the leading `#`).
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn hashtag_timeline(
        &self,
        hashtag: &str,
        params: &TimelineParams,
    ) -> Result<Vec<Status<'static>>> {
        self.get_json(&format!("/api/v1/timelines/tag/{hashtag}"), &params.query())
            .await
    }

    /// `GET /api/v1/timelines/group/:group_id`: a group's timeline (Truth Social specific).
    ///
    /// # Errors
    ///
    /// Any request error.
    pub async fn group_timeline(
        &self,
        group_id: &str,
        params: &TimelineParams,
    ) -> Result<Vec<Status<'static>>> {
        self.get_json(
            &format!("/api/v1/timelines/group/{group_id}"),
            &params.query(),
        )
        .await
    }

    // ── Raw / generic access ───────────────────────────────────────────────────

    /// Issue a `GET` to an arbitrary API path and deserialize the JSON body as `T`.
    ///
    /// This backs the typed endpoints and is public so callers can reach endpoints this crate does
    /// not yet model: deserialize into [`serde_json::Value`] for fully dynamic access, or into any
    /// type that implements [`serde::de::DeserializeOwned`].
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-success status, [`Error::Decode`] if the body does not match `T`,
    /// or [`Error::Transport`] / [`Error::Url`].
    pub async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let body = self.get_text(path, query).await?;
        serde_json::from_str(&body).map_err(|source| Error::Decode {
            target: std::any::type_name::<T>(),
            source,
        })
    }

    /// Issue a `GET` and return the raw response body, attaching the bearer token when present.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-success status, or [`Error::Transport`] / [`Error::Url`].
    pub async fn get_text(&self, path: &str, query: &[(&str, String)]) -> Result<String> {
        let url = self.base.join(path)?;
        let response = self.prepare(self.http.get(url).query(query)).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if status.is_success() {
            Ok(body)
        } else {
            Err(Error::Api { status, body })
        }
    }

    /// Attach the bearer token (when set) and the `Cookie` header (when set) to a request.
    fn prepare(&self, mut request: wreq::RequestBuilder) -> wreq::RequestBuilder {
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(cookie) = &self.cookie_header {
            request = request.header("cookie", cookie);
        }
        request
    }
}

/// Builder for a [`Client`].
#[derive(Clone, Debug)]
pub struct ClientBuilder {
    base_url: String,
    token: Option<String>,
    emulation: Emulation,
    user_agent: Option<String>,
    cookies: Vec<(String, String)>,
    timeout: Option<Duration>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            token: None,
            emulation: DEFAULT_EMULATION,
            user_agent: Some(DEFAULT_USER_AGENT.to_owned()),
            cookies: Vec::new(),
            timeout: Some(Duration::from_secs(30)),
        }
    }
}

impl ClientBuilder {
    /// Override the API origin (default [`DEFAULT_BASE_URL`]). Useful for tests against a mock
    /// server.
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Pre-load a bearer token.
    #[must_use]
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Apply a captured [`Config`]: its User-Agent, `cf_clearance` (and `__cf_bm`) cookies, and
    /// bearer token (if any). Produced by the `ts-login` utility.
    #[must_use]
    pub fn config(mut self, config: &Config) -> Self {
        self = self
            .user_agent(config.user_agent.clone())
            .cf_clearance(config.cf_clearance.clone());
        if let Some(cf_bm) = &config.cf_bm {
            self = self.cookie("__cf_bm", cf_bm.clone());
        }
        if let Some(token) = &config.token {
            self = self.token(token.clone());
        }
        self
    }

    /// Override the browser to emulate (default [`DEFAULT_EMULATION`]). Pick a different
    /// [`Emulation`] if Cloudflare starts flagging the default.
    #[must_use]
    pub const fn emulation(mut self, emulation: Emulation) -> Self {
        self.emulation = emulation;
        self
    }

    /// Override the `User-Agent` header (default [`DEFAULT_USER_AGENT`]). It must match the browser
    /// a `cf_clearance` cookie was obtained from, since Cloudflare binds clearance to the agent.
    #[must_use]
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Add a cookie sent on every request (e.g. `("cf_clearance", "…")`). Repeated calls
    /// accumulate.
    #[must_use]
    pub fn cookie(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.cookies.push((name.into(), value.into()));
        self
    }

    /// Convenience for the Cloudflare clearance cookie.
    ///
    /// When the impersonating client is itself challenged (typically when running from a flagged
    /// IP), solve the challenge once in a real browser and pass its `cf_clearance` cookie here,
    /// together with the matching [`user_agent`](Self::user_agent): Cloudflare binds clearance to
    /// the IP and User-Agent, so the client must run from the same network and present the same
    /// agent.
    #[must_use]
    pub fn cf_clearance(self, value: impl Into<String>) -> Self {
        self.cookie("cf_clearance", value)
    }

    /// Set a per-request timeout (default 30s). `None` disables it.
    #[must_use]
    pub const fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build the [`Client`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Url`] if the base URL is invalid, or [`Error::Transport`] if the HTTP
    /// client cannot be initialized.
    pub fn build(self) -> Result<Client> {
        let base = Url::parse(&self.base_url)?;

        let mut builder = wreq::Client::builder().emulation(self.emulation);
        if let Some(user_agent) = self.user_agent {
            builder = builder.user_agent(user_agent);
        }
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }

        // Render the accumulated cookies into a single `Cookie` header value (`a=1; b=2`).
        let cookie_header = if self.cookies.is_empty() {
            None
        } else {
            Some(
                self.cookies
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        };

        Ok(Client {
            http: builder.build()?,
            base,
            token: self.token,
            cookie_header,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let client = Client::new().expect("build default client");
        assert_eq!(client.base.as_str(), "https://truthsocial.com/");
        assert!(!client.is_authenticated());
        assert_eq!(client.token(), None);
    }

    #[test]
    fn token_round_trips() {
        let mut client = Client::with_token("abc").expect("build");
        assert!(client.is_authenticated());
        assert_eq!(client.token(), Some("abc"));
        client.set_token("def");
        assert_eq!(client.token(), Some("def"));
    }

    #[test]
    fn invalid_base_url_is_rejected() {
        let result = Client::builder().base_url("not a url").build();
        assert!(matches!(result, Err(Error::Url(_))));
    }
}
