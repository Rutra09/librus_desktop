//! Shared HTTP client defaults.
//!
//! `reqwest` applies *no* timeout unless one is asked for, so a stalled TLS
//! handshake or a dropped response hangs the request forever. That is how one
//! refresh ended up logging `metadata ready` after 651 s: the very first
//! request never came back, and nothing in the app was willing to give up on
//! it. Every client now goes through this module so the whole refresh has a
//! hard upper bound.

use std::time::Duration;

use reqwest::Client;

use super::constants::LIBRUS_USER_AGENT;

/// How long to wait for the TCP/TLS handshake.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a single request may take, end to end.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// A `ClientBuilder` preloaded with the app's timeouts.
pub fn builder() -> reqwest::ClientBuilder {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
}

/// `builder()` plus the Librus mobile user agent — the common case.
pub fn client() -> Client {
    builder()
        .user_agent(LIBRUS_USER_AGENT)
        .build()
        .expect("Failed to create HTTP client")
}
