//! An asynchronous client for the [Truth Social](https://truthsocial.com) HTTP API.
//!
//! Responses use the shared [`truthsocial::model`] types for both live API data and archives.
//!
//! # Authentication
//!
//! Supply a bearer token with [`Client::with_token`](client::Client::with_token), or use
//! [`Client::login`](client::Client::login) to exchange an application's client ID and secret plus
//! a user's username and password for a token. [`Credentials`](auth::Credentials) accepts these
//! values directly or reads them from the environment.
//!
//! ```no_run
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use truthsocial_api::{auth::Credentials, client::Client, types::TimelineParams};
//!
//! let mut client = Client::new()?;
//! client.login(&Credentials::from_env()?).await?;
//!
//! let account = client.lookup_account("realDonaldTrump").await?;
//! let statuses = client
//!     .account_statuses(&account.id.to_string(), &TimelineParams::default().limit(5))
//!     .await?;
//! println!("fetched {} statuses", statuses.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Browser sessions
//!
//! The client emulates Firefox by default, but requests can still receive a Cloudflare challenge.
//! Run `cargo run -p truthsocial-api-cli -- setup` to capture a browser session, then load the
//! resulting file with [`Config::load`](config::Config::load) and apply it with
//! [`ClientBuilder::config`](client::ClientBuilder::config). Setup captures a bearer token when
//! it can find one; otherwise, supply a token or call `login` separately.
//!
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod types;
