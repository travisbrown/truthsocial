//! An asynchronous client for the [Truth Social](https://truthsocial.com) HTTP API.
//!
//! Truth Social is a fork of [Mastodon](https://docs.joinmastodon.org/api/), so most endpoints
//! follow the Mastodon REST conventions (with some Truth-Social-specific additions such as groups).
//! Responses deserialize into the shared [`truthsocial::model`] types, so a fetched status is the
//! same `Status` type used elsewhere in this workspace.
//!
//! # Authentication
//!
//! Truth Social authenticates with the OAuth 2.0 *password* grant. You need four values (an
//! application `client_id` and `client_secret`, plus a user `username` and `password`), which you
//! supply via [`Credentials`](auth::Credentials) (directly or from the environment). The client
//! never stores these in source.
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
//! # Unauthenticated access
//!
//! A few endpoints may answer without a token, but in practice Truth Social requires authentication
//! for almost everything and fronts the whole site with Cloudflare. Expect unauthenticated requests
//! (and sometimes even authenticated ones from a non-browser TLS stack) to receive a `403`
//! challenge ([`Error::Api`](error::Error::Api)). If that happens, authenticate, and if it persists
//! you may need a client that mimics a browser's TLS fingerprint (e.g. an impersonating `reqwest`
//! fork); the [`Client::builder`](client::Client::builder) `user_agent` override is the first thing
//! to try.
#![warn(clippy::all, clippy::pedantic, clippy::nursery, rust_2018_idioms)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod types;
