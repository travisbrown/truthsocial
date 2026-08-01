//! Persisted session configuration: the User-Agent and Cloudflare cookies (and optionally a bearer
//! token) needed to reach the API from a flagged network.
//!
//! Produced by the `setup` command of the `truthsocial-api-cli` crate and consumed via
//! [`ClientBuilder::config`](crate::client::ClientBuilder::config).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use archivindex_publication::{Policy, Publication};
use serde::{Deserialize, Serialize};

/// The default config file name, in the current directory.
pub const DEFAULT_CONFIG_FILE: &str = "creds.toml";

/// A captured browser session: the values that let an HTTP client masquerade as the browser that
/// solved Cloudflare.
///
/// `cf_clearance` is bound to the [`user_agent`](Self::user_agent) and the originating IP, so a
/// client must present the same agent and run from the same network the session was captured on.
#[derive(Clone, Default, Deserialize, Serialize)]
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

impl std::fmt::Debug for Config {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Config")
            .field("user_agent", &self.user_agent)
            .field("cf_clearance", &"<redacted>")
            .field("cf_bm", &self.cf_bm.as_ref().map(|_| "<redacted>"))
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
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

    /// Atomically replace a TOML config file with this session.
    ///
    /// Stages a new file in the existing parent directory, then syncs and publishes it. On Unix the
    /// new file is owner-only (`0600`, subject to a more restrictive umask), even when replacing a
    /// file with broader permissions. A destination symlink is replaced, leaving its target
    /// untouched; permissions of the old file are not copied. Failures before publication leave the
    /// previous config unchanged.
    ///
    /// Directory synchronization is performed on Unix. If it fails after publication, the new
    /// config remains visible. The publication error is retained inside the [`ConfigError::Io`]
    /// source and can be downcast to [`archivindex_publication::Error`] to distinguish that
    /// outcome. Newly created ancestor directories are not synced.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Serialize`] if serialization fails, or [`ConfigError::Io`] if the file cannot
    /// be written.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let serialized = toml::to_string_pretty(self)?;
        let mut output = Publication::new(path, Policy::Replace)?;
        output.write_all(serialized.as_bytes())?;
        output.publish().map_err(std::io::Error::from)?;
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
    fn debug_redacts_session_values() {
        let config = Config {
            user_agent: "visible agent".to_owned(),
            cf_clearance: "sentinel-clearance".to_owned(),
            cf_bm: Some("sentinel-bm".to_owned()),
            token: Some("sentinel-token".to_owned()),
        };
        for debug in [format!("{config:?}"), format!("{config:#?}")] {
            assert!(debug.contains("visible agent"));
            assert!(debug.contains("<redacted>"));
            for secret in [
                &config.cf_clearance,
                config.cf_bm.as_ref().unwrap(),
                config.token.as_ref().unwrap(),
            ] {
                assert!(!debug.contains(secret));
            }
        }
        let debug = format!("{:?}", Config::default());
        assert!(debug.contains("cf_bm: None"));
        assert!(debug.contains("token: None"));
    }

    #[cfg(unix)]
    #[test]
    fn saves_create_private_files_and_replace_old_permissions_and_links()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::{PermissionsExt as _, symlink};

        let directory = tempfile::tempdir()?;
        let path = directory.path().join("creds.toml");
        let config = Config {
            cf_clearance: "new-clearance".to_owned(),
            ..Config::default()
        };
        config.save(&path)?;
        assert_eq!(std::fs::metadata(&path)?.permissions().mode() & 0o077, 0);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
        let original = directory.path().join("original.toml");
        std::fs::hard_link(&path, &original)?;
        let original_bytes = std::fs::read(&original)?;
        let replacement = Config {
            cf_clearance: "replacement".to_owned(),
            ..Config::default()
        };
        replacement.save(&path)?;
        assert_eq!(std::fs::metadata(&path)?.permissions().mode() & 0o077, 0);
        assert_eq!(std::fs::read(&original)?, original_bytes);
        assert_eq!(Config::load(&path)?.cf_clearance, "replacement");

        std::fs::remove_file(&path)?;
        symlink(&original, &path)?;
        replacement.save(&path)?;
        assert!(!std::fs::symlink_metadata(&path)?.file_type().is_symlink());
        assert_eq!(std::fs::read(&original)?, original_bytes);
        assert_eq!(std::fs::metadata(&path)?.permissions().mode() & 0o077, 0);
        assert_eq!(std::fs::read_dir(directory.path())?.count(), 2);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn failed_save_preserves_existing_session() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir()?;
        let path = directory.path().join("creds.toml");
        Config::default().save(&path)?;
        let original = std::fs::read(&path)?;
        let permissions = std::fs::metadata(directory.path())?.permissions();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o500))?;
        // Privileged test runners can bypass directory permissions; this failure mode can only be
        // exercised when the filesystem enforces them for this process.
        let probe = std::fs::File::create(directory.path().join("permission-probe"));
        if probe.is_ok() {
            std::fs::set_permissions(directory.path(), permissions)?;
            return Ok(());
        }
        let config = Config {
            cf_clearance: "replacement".to_owned(),
            ..Config::default()
        };
        let result = config.save(&path);
        std::fs::set_permissions(directory.path(), permissions)?;
        assert!(matches!(result, Err(ConfigError::Io(_))));
        assert_eq!(std::fs::read(&path)?, original);
        assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
        Ok(())
    }

    #[test]
    fn failed_publication_cleans_up_staging() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("creds.toml");
        std::fs::create_dir(&path)?;
        let existing = path.join("keep");
        std::fs::write(&existing, b"existing")?;
        assert!(matches!(
            Config::default().save(&path),
            Err(ConfigError::Io(_))
        ));
        assert_eq!(std::fs::read(existing)?, b"existing");
        assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
        Ok(())
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
