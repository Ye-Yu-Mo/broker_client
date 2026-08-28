//! Broker client library for the TW and A-share stock servers.
//!
//! This crate provides a shared async HTTP foundation (`ClientConfig`,
//! `HttpClient`, `Error`) plus feature-gated server clients:
//!
//! - `TwClient` under the `client-tw` feature (default).
//! - `AClient` under the `client-a` feature (default).
//! - WebSocket dependencies/APIs under the `ws` feature (not yet wired in M1).

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod http;
pub mod response;

#[cfg(feature = "client-tw")]
pub use client::TwClient;

#[cfg(feature = "client-a")]
pub use client::AClient;

pub use auth::AuthMethod;
pub use config::ClientConfig;
pub use error::{Error, Result};
