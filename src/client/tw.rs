//! Taiwan stock-broker client skeleton.

use serde_json::Value;

use crate::config::ClientConfig;
use crate::error::Result;
use crate::http::HttpClient;

/// Client for the stock-broker-tw-server.
///
/// The default base URL is `http://127.0.0.1:8000`.
#[derive(Debug, Clone)]
pub struct TwClient {
    http: HttpClient,
}

impl TwClient {
    /// Creates a client with the default TW server configuration.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            http: HttpClient::new(config),
        }
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &ClientConfig {
        self.http.config()
    }

    /// Calls `GET /health` and returns the parsed health JSON.
    pub async fn health(&self) -> Result<Value> {
        self.http.get_json("/health").await
    }

    /// Calls `GET /metrics` and returns the raw Prometheus text.
    pub async fn metrics(&self) -> Result<String> {
        self.http.get_text("/metrics").await
    }
}

impl Default for TwClient {
    fn default() -> Self {
        Self::new(ClientConfig::tw_default())
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::TwClient;

    #[tokio::test]
    async fn default_base_url_matches_docs() {
        let client = TwClient::default();
        assert_eq!(client.config().base_url, "http://127.0.0.1:8000");
    }

    #[tokio::test]
    async fn health_hits_health_path() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status":"ok"})),
            )
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let value = client.health().await.unwrap();
        assert_eq!(value["status"], "ok");
    }

    #[tokio::test]
    async fn metrics_returns_raw_text() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_string("http_requests_total 1\n"))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        assert_eq!(client.metrics().await.unwrap(), "http_requests_total 1\n");
    }
}
