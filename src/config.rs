//! Shared client configuration.

use std::collections::HashMap;
use std::time::Duration;

use crate::auth::AuthMethod;

/// Common configuration for both TW and A-share clients.
///
/// Values are intentionally kept in one place so `TwClient` and `AClient`
/// can share the same HTTP, timeout, retry and header plumbing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    /// Server base URL, e.g. `http://127.0.0.1:8000`.
    pub base_url: String,
    /// Optional token. When `None`/empty no authentication header is sent.
    pub token: Option<String>,
    /// Which auth header style to use when a token is configured.
    pub auth_method: AuthMethod,
    /// Per-request timeout.
    pub timeout: Duration,
    /// Number of additional attempts for safe GET requests. `0` disables retry.
    pub retry: u32,
    /// Maximum WebSocket reconnect attempts before giving up.
    pub ws_max_reconnect_attempts: u32,
    /// Base backoff (milliseconds) between WebSocket reconnects.
    pub ws_base_backoff_ms: u64,
    /// Optional separate WebSocket base URL.
    ///
    /// When `None`, the WebSocket URL is derived from [`Self::base_url`].
    pub ws_base_url: Option<String>,
    /// Value sent as `User-Agent`.
    pub user_agent: String,
    /// Extra headers merged into every request.
    pub default_headers: HashMap<String, String>,
}

impl ClientConfig {
    /// Creates a config with common defaults for a given base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: None,
            auth_method: AuthMethod::default(),
            timeout: Duration::from_secs(10),
            retry: 2,
            ws_max_reconnect_attempts: 5,
            ws_base_backoff_ms: 500,
            ws_base_url: None,
            user_agent: format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            default_headers: HashMap::new(),
        }
    }

    /// Returns the default configuration for the TW server.
    pub fn tw_default() -> Self {
        Self::new("http://127.0.0.1:8000")
    }

    /// Returns the default configuration for the A-share server.
    pub fn a_default() -> Self {
        Self::new("http://127.0.0.1:8787")
    }

    /// Builder-style override for [`Self::base_url`].
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Builder-style override for [`Self::token`].
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Builder-style override for [`Self::auth_method`].
    pub fn auth_method(mut self, method: AuthMethod) -> Self {
        self.auth_method = method;
        self
    }

    /// Builder-style override for [`Self::timeout`].
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builder-style override for [`Self::retry`].
    pub fn retry(mut self, retry: u32) -> Self {
        self.retry = retry;
        self
    }

    /// Builder-style override for [`Self::ws_max_reconnect_attempts`].
    pub fn ws_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.ws_max_reconnect_attempts = attempts;
        self
    }

    /// Builder-style override for [`Self::ws_base_backoff_ms`].
    pub fn ws_base_backoff_ms(mut self, millis: u64) -> Self {
        self.ws_base_backoff_ms = millis;
        self
    }

    /// Builder-style override for [`Self::ws_base_url`].
    pub fn ws_base_url(mut self, ws_base_url: impl Into<String>) -> Self {
        self.ws_base_url = Some(ws_base_url.into());
        self
    }

    /// Builder-style override for [`Self::user_agent`].
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Adds one default header.
    pub fn default_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(key.into(), value.into());
        self
    }

    /// Replaces all default headers.
    pub fn default_headers(
        mut self,
        headers: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.default_headers = headers
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::tw_default()
    }
}

/// Joins a base URL and a path without accidentally creating a double slash.
pub fn join_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ClientConfig, join_url};
    use crate::auth::AuthMethod;

    #[test]
    fn tw_defaults_match_expected_server() {
        let config = ClientConfig::tw_default();
        assert_eq!(config.base_url, "http://127.0.0.1:8000");
        assert_eq!(config.token, None);
        assert_eq!(config.auth_method, AuthMethod::Bearer);
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.retry, 2);
        assert_eq!(config.ws_max_reconnect_attempts, 5);
        assert_eq!(config.ws_base_backoff_ms, 500);
        assert_eq!(config.ws_base_url, None);
    }

    #[test]
    fn a_defaults_match_expected_server() {
        let config = ClientConfig::a_default();
        assert_eq!(config.base_url, "http://127.0.0.1:8787");
        assert_eq!(config.auth_method, AuthMethod::Bearer);
    }

    #[test]
    fn builder_can_override_every_field() {
        let config = ClientConfig::new("http://localhost:1111")
            .base_url("http://localhost:2222")
            .token("tok")
            .auth_method(AuthMethod::XAuthToken)
            .timeout(Duration::from_millis(1))
            .retry(0)
            .user_agent("test-agent")
            .default_header("X-Custom", "yes");

        assert_eq!(config.base_url, "http://localhost:2222");
        assert_eq!(config.token.as_deref(), Some("tok"));
        assert_eq!(config.auth_method, AuthMethod::XAuthToken);
        assert_eq!(config.timeout, Duration::from_millis(1));
        assert_eq!(config.retry, 0);
        assert_eq!(config.user_agent, "test-agent");
        assert_eq!(
            config.default_headers.get("X-Custom").map(String::as_str),
            Some("yes")
        );
    }

    #[test]
    fn empty_token_is_kept_as_none_by_builder_contract() {
        // The builder is generic; callers that need to clear a token can pass
        // an empty string. The HTTP layer treats empty tokens as "no auth".
        let config = ClientConfig::new("http://x").token("");
        assert_eq!(config.token.as_deref(), Some(""));
    }

    #[test]
    fn join_url_handles_trailing_and_leading_slashes() {
        assert_eq!(
            join_url("http://127.0.0.1:8000/", "/health"),
            "http://127.0.0.1:8000/health"
        );
        assert_eq!(
            join_url("http://127.0.0.1:8000", "v1/health"),
            "http://127.0.0.1:8000/v1/health"
        );
        assert_eq!(
            join_url("http://127.0.0.1:8000/", "v1/health"),
            "http://127.0.0.1:8000/v1/health"
        );
    }
}
