//! Core types for the Truth Social API.
//!
//! This crate provides Rust bindings for deserializing (and serializing) Truth Social status (post)
//! data and related types like accounts, media attachments, mentions, and tags.
//!
//! All model types live in [`model`]. These same types double as the content model for Wayback
//! Machine archive snapshots; that integration lives in the separate `truthsocial-wbm` crate.
pub mod model;

/// Classification of Truth Social API URLs.
pub mod url;
