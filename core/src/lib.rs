//! Core types for the Truth Social API.
//!
//! This crate provides Rust bindings for deserializing (and serializing) Truth Social status (post)
//! data and related types like accounts, media attachments, mentions, and tags.
//!
//! All model types live in [`model`]. These same types double as the content model for Wayback
//! Machine archive snapshots; that integration lives in the separate `truthsocial-wbm` crate.
#![warn(clippy::all, clippy::pedantic, clippy::nursery, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod model;
