//! OAuth 2.0 password-grant authentication.
//!
//! Exchanges an application's client ID and secret plus a user's username and password for a
//! bearer token at `POST /oauth/token`. Supply credentials with [`Credentials::new`] or
//! [`Credentials::from_env`].

/// Environment variable holding the OAuth application client ID.
pub const CLIENT_ID_ENV: &str = "TRUTHSOCIAL_CLIENT_ID";
/// Environment variable holding the OAuth application client secret.
pub const CLIENT_SECRET_ENV: &str = "TRUTHSOCIAL_CLIENT_SECRET";
/// Environment variable holding the account username (email or handle).
pub const USERNAME_ENV: &str = "TRUTHSOCIAL_USERNAME";
/// Environment variable holding the account password.
pub const PASSWORD_ENV: &str = "TRUTHSOCIAL_PASSWORD";

/// The default OAuth scope requested at login.
pub const DEFAULT_SCOPE: &str = "read";

/// Credentials for the OAuth 2.0 password grant.
#[derive(Clone)]
pub struct Credentials {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
    scope: String,
}

impl Credentials {
    /// Build credentials from an application ID/secret and a user's username and password, using
    /// the [`DEFAULT_SCOPE`].
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            username: username.into(),
            password: password.into(),
            scope: DEFAULT_SCOPE.to_owned(),
        }
    }

    /// Load credentials from the [`CLIENT_ID_ENV`], [`CLIENT_SECRET_ENV`], [`USERNAME_ENV`], and
    /// [`PASSWORD_ENV`] environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`std::env::VarError`] if a variable is unset or contains invalid Unicode.
    /// The error does not identify the variable name.
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Self::new(
            std::env::var(CLIENT_ID_ENV)?,
            std::env::var(CLIENT_SECRET_ENV)?,
            std::env::var(USERNAME_ENV)?,
            std::env::var(PASSWORD_ENV)?,
        ))
    }

    /// Override the requested OAuth scope (the default is [`DEFAULT_SCOPE`]).
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = scope.into();
        self
    }

    /// The form fields for the `POST /oauth/token` password-grant request.
    pub(crate) fn form_fields(&self) -> [(&'static str, &str); 6] {
        [
            ("grant_type", "password"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("username", &self.username),
            ("password", &self.password),
            ("scope", &self.scope),
        ]
    }
}

impl std::fmt::Debug for Credentials {
    /// Redacts the client secret and password from debug output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("scope", &self.scope)
            .finish()
    }
}

/// The subset of the `POST /oauth/token` response we consume.
///
/// `pub` rather than `pub(crate)`: the enclosing module is private, so this stays crate-internal
/// regardless, and `pub` avoids the `redundant_pub_crate` lint.
#[derive(Debug, serde::Deserialize)]
pub struct TokenResponse {
    /// The bearer token returned by the OAuth server.
    pub access_token: String,
}
