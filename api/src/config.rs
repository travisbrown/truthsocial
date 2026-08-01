//! Persisted session configuration: the User-Agent and Cloudflare cookies (and optionally a bearer
//! token) needed to reach the API from a flagged network.
//!
//! Produced by the `setup` command of the `truthsocial-api-cli` crate and consumed via
//! [`ClientBuilder::config`](crate::client::ClientBuilder::config).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The default config file name, in the current directory.
pub const DEFAULT_CONFIG_FILE: &str = "creds.toml";

/// A captured browser session: the values that let an HTTP client masquerade as the browser that
/// solved Cloudflare.
///
/// `cf_clearance` is bound to the [`user_agent`](Self::user_agent) and the originating IP, so a
/// client must present the same agent and run from the same network the session was captured on.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The browser User-Agent the cookies are bound to.
    pub user_agent: String,
    /// The Cloudflare `cf_clearance` cookie value.
    pub cf_clearance: String,
    /// The Cloudflare `__cf_bm` bot-management cookie, if captured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cf_bm: Option<String>,
    /// The OAuth bearer token, if captured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl Config {
    /// The default config path ([`DEFAULT_CONFIG_FILE`] in the current directory).
    #[must_use]
    pub fn default_path() -> PathBuf {
        PathBuf::from(DEFAULT_CONFIG_FILE)
    }

    /// Load a config from a TOML file.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Io`] if the file cannot be read, or [`ConfigError::Parse`] if it is not valid
    /// TOML for this schema.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }

    /// Write this config to a TOML file.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Serialize`] if serialization fails, or [`ConfigError::Io`] if the file cannot
    /// be written.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// An error loading or saving a [`Config`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file could not be read or written.
    #[error("config file I/O error")]
    Io(#[from] std::io::Error),
    /// The file was not valid TOML for the [`Config`] schema.
    #[error("failed to parse config")]
    Parse(#[from] toml::de::Error),
    /// The config could not be serialized to TOML.
    #[error("failed to serialize config")]
    Serialize(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_through_toml() {
        let config = Config {
            user_agent: "Mozilla/5.0 (X11; Linux x86_64; rv:151.0) Gecko/20100101 Firefox/151.0"
                .to_owned(),
            cf_clearance: "abc.def-123".to_owned(),
            cf_bm: Some("bm-cookie".to_owned()),
            token: Some("tok123".to_owned()),
        };

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(Config::default_path());
        config.save(&path).expect("save");
        let loaded = Config::load(&path).expect("load");

        assert_eq!(loaded.user_agent, config.user_agent);
        assert_eq!(loaded.cf_clearance, config.cf_clearance);
        assert_eq!(loaded.cf_bm, config.cf_bm);
        assert_eq!(loaded.token, config.token);
    }

    #[test]
    fn optional_fields_are_omitted_when_absent() {
        let config = Config {
            user_agent: "ua".to_owned(),
            cf_clearance: "clearance".to_owned(),
            cf_bm: None,
            token: None,
        };
        let toml = toml::to_string_pretty(&config).expect("serialize");
        assert!(!toml.contains("cf_bm"));
        assert!(!toml.contains("token"));
    }
}
