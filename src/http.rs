//! Shared asynchronous HTTP client used by both server clients.

use std::time::Duration;

use reqwest::Method;
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::config::{ClientConfig, join_url};
use crate::error::{Error, Result};
use crate::response::http_error;

/// Thin wrapper around `reqwest::Client` that centralises headers, URL
/// building, JSON handling, timeout and safe GET retry.
#[derive(Debug, Clone)]
pub struct HttpClient {
    config: ClientConfig,
    client: reqwest::Client,
}

impl HttpClient {
    /// Creates a client from a shared [`ClientConfig`].
    ///
    /// `reqwest::Client` is cheap to clone and keeps a connection pool alive.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Returns the underlying configuration.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Sends a GET request and decodes the JSON response.
    pub async fn get_json<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.send_json(Method::GET, path, &[], None, None).await
    }

    /// Sends a GET request with query parameters and decodes the JSON response.
    pub async fn get_json_with_query<T>(&self, path: &str, query: &[(&str, &str)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.send_json(Method::GET, path, query, None, None).await
    }

    /// Sends a POST request with a JSON body and decodes the JSON response.
    pub async fn post_json<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let body = serde_json::to_string(body)
            .map_err(|e| Error::Transport(format!("serialize request body: {e}")))?;
        self.send_json(Method::POST, path, &[], Some(body), None)
            .await
    }

    /// Sends a GET request and returns the raw response body as text.
    pub async fn get_text(&self, path: &str) -> Result<String> {
        self.send_raw(Method::GET, path, &[], None).await
    }

    /// Low-level request helper used by higher-level methods.
    ///
    /// `request_id` can be supplied by callers; when `None` a UUID is generated.
    pub async fn send_raw(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        request_id: Option<&str>,
    ) -> Result<String> {
        self.send_raw_inner(method, path, query, None, request_id)
            .await
    }

    async fn send_json<T>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<String>,
        request_id: Option<&str>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let text = self
            .send_raw_inner(method, path, query, body, request_id)
            .await?;
        serde_json::from_str(&text).map_err(|e| Error::decode(text, e))
    }

    async fn send_raw_inner(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<String>,
        request_id: Option<&str>,
    ) -> Result<String> {
        let url = join_url(&self.config.base_url, path);
        let url = reqwest::Url::parse(&url).map_err(|_| Error::InvalidUrl(url.clone()))?;

        let max_attempts = if method == Method::GET {
            self.config.retry.saturating_add(1)
        } else {
            1
        };

        let mut last_error = None;
        for attempt in 0..max_attempts {
            match self
                .execute_attempt(method.clone(), url.clone(), query, body.clone(), request_id)
                .await
            {
                Ok(text) => return Ok(text),
                Err(err) => {
                    let retryable =
                        method == Method::GET && attempt + 1 < max_attempts && err.is_retryable();
                    if retryable {
                        last_error = Some(err);
                        self.sleep_before_retry(attempt).await;
                    } else {
                        return Err(err);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Transport("request failed".to_owned())))
    }

    async fn execute_attempt(
        &self,
        method: Method,
        url: reqwest::Url,
        query: &[(&str, &str)],
        body: Option<String>,
        request_id: Option<&str>,
    ) -> Result<String> {
        let mut builder = self
            .client
            .request(method, url)
            .query(query)
            .timeout(self.config.timeout);

        // Default headers first, then built-in headers so the client's
        // authentication/request-ID/user-agent contract cannot be accidentally
        // overridden by user default headers.
        for (key, value) in &self.config.default_headers {
            builder = builder.header(key, value);
        }

        builder = builder.header(reqwest::header::USER_AGENT, &self.config.user_agent);
        builder = builder.header(
            "x-request-id",
            request_id.unwrap_or(&Uuid::new_v4().to_string()),
        );

        if let Some(token) = self.config.token.as_deref() {
            if !token.is_empty() {
                let method = self.config.auth_method;
                builder = builder.header(method.header_name(), method.header_value(token));
            }
        }

        if let Some(body) = body {
            builder = builder
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body);
        }

        let response = builder.send().await?;
        let status = response.status().as_u16();
        let text = response.text().await.map_err(Error::from)?;

        if (200..300).contains(&status) {
            Ok(text)
        } else {
            Err(http_error(status, text))
        }
    }

    async fn sleep_before_retry(&self, attempt: u32) {
        // Simple linear backoff: 50ms, 100ms, ...
        let millis = 50_u64 * u64::from(attempt + 1);
        tokio::time::sleep(Duration::from_millis(millis)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::Method;
    use serde_json::json;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::HttpClient;
    use crate::auth::AuthMethod;
    use crate::config::ClientConfig;
    use crate::response::TwEnvelope;

    #[tokio::test]
    async fn get_json_uses_url_headers_and_parses_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/health"))
            .and(header("authorization", "Bearer secret"))
            .and(header("user-agent", "test-agent"))
            .and(header("x-custom", "yes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"ok"})))
            .mount(&server)
            .await;

        let config = ClientConfig::new(server.uri())
            .token("secret")
            .user_agent("test-agent")
            .default_header("X-Custom", "yes");
        let client = HttpClient::new(config);

        let value: serde_json::Value = client.get_json("/v1/health").await.unwrap();
        assert_eq!(value["status"], "ok");
    }

    #[tokio::test]
    async fn x_auth_token_header_is_supported() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ping"))
            .and(header("x-auth-token", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let config = ClientConfig::new(server.uri())
            .token("secret")
            .auth_method(AuthMethod::XAuthToken);
        let client = HttpClient::new(config);
        let text = client.get_text("/ping").await.unwrap();
        assert_eq!(text, "ok");
    }

    #[tokio::test]
    async fn post_json_sends_content_type_and_exact_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders"))
            .and(header("content-type", "application/json"))
            .and(body_json(
                json!({"client_order_id": "C001", "symbol": "512100"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "accepted"})))
            .mount(&server)
            .await;

        let client = HttpClient::new(ClientConfig::new(server.uri()).retry(0));
        let value: serde_json::Value = client
            .post_json(
                "/orders",
                &json!({"client_order_id": "C001", "symbol": "512100"}),
            )
            .await
            .unwrap();
        assert_eq!(value["status"], "accepted");
    }

    #[tokio::test]
    async fn empty_token_does_not_send_auth_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ping"))
            .and(header("authorization", "Bearer secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("should-not-happen"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ping"))
            .respond_with(ResponseTemplate::new(200).set_body_string("no-auth"))
            .mount(&server)
            .await;

        let config = ClientConfig::new(server.uri()).token("");
        let client = HttpClient::new(config);
        let text = client.get_text("/ping").await.unwrap();
        assert_eq!(text, "no-auth");
    }

    #[tokio::test]
    async fn request_id_is_present_even_when_not_supplied() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/id"))
            .and(header("x-request-id", "custom-id"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let config = ClientConfig::new(server.uri());
        let client = HttpClient::new(config);
        let text = client
            .send_raw(Method::GET, "/id", &[], Some("custom-id"))
            .await
            .unwrap();
        assert_eq!(text, "ok");
    }

    #[tokio::test]
    async fn post_requests_are_not_retried_on_5xx() {
        let server = MockServer::start().await;
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();

        Mock::given(method("POST"))
            .and(path("/orders"))
            .respond_with(move |_req: &wiremock::Request| {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ResponseTemplate::new(503).set_body_json(json!({
                    "detail": {"code": "CIRCUIT_OPEN", "message": "open", "detail": {}}
                }))
            })
            .mount(&server)
            .await;

        let config = ClientConfig::new(server.uri()).retry(3);
        let client = HttpClient::new(config);
        let err = client
            .post_json::<serde_json::Value, _>("/orders", &json!({"x": 1}))
            .await
            .unwrap_err();
        assert!(matches!(err, crate::error::Error::Api { .. }));
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn get_requests_retry_on_503_and_then_succeed() {
        let server = MockServer::start().await;
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();

        Mock::given(method("GET"))
            .and(path("/retry"))
            .respond_with(move |_req: &wiremock::Request| {
                let n = counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n == 0 {
                    ResponseTemplate::new(503).set_body_string("busy")
                } else {
                    ResponseTemplate::new(200).set_body_json(json!({"ok": true}))
                }
            })
            .mount(&server)
            .await;

        let config = ClientConfig::new(server.uri()).retry(2);
        let client = HttpClient::new(config);
        let value: serde_json::Value = client.get_json("/retry").await.unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn invalid_url_maps_to_error() {
        let config = ClientConfig::new("not a url");
        let client = HttpClient::new(config);
        let err = client.get_text("/x").await.unwrap_err();
        assert!(matches!(err, crate::error::Error::InvalidUrl(_)));
    }

    #[tokio::test]
    async fn request_timeout_maps_to_timeout_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(200)))
            .mount(&server)
            .await;

        let config = ClientConfig::new(server.uri())
            .timeout(Duration::from_millis(20))
            .retry(0);
        let client = HttpClient::new(config);
        let err = client.get_text("/slow").await.unwrap_err();
        assert!(matches!(err, crate::error::Error::Timeout));
    }

    #[tokio::test]
    async fn invalid_json_maps_to_decode_error_with_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bad"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
            .mount(&server)
            .await;

        let client = HttpClient::new(ClientConfig::new(server.uri()).retry(0));
        let err = client
            .get_json::<serde_json::Value>("/bad")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::error::Error::Decode { body, .. } if body == "not-json"
        ));
    }

    #[tokio::test]
    async fn tw_envelope_is_parsed_through_http_client() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {"status": "ok"}
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new(ClientConfig::new(server.uri()));
        let envelope: TwEnvelope<serde_json::Value> = client.get_json("/api/health").await.unwrap();
        let data = envelope.into_data().unwrap();
        assert_eq!(data["status"], "ok");
    }

    #[tokio::test]
    async fn tw_error_envelope_is_mapped_through_http_client() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "detail": {
                    "code": "ORDER_NOT_FOUND",
                    "message": "missing order",
                    "detail": {"id": 1}
                }
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new(ClientConfig::new(server.uri()).retry(0));
        let err = client
            .get_json::<serde_json::Value>("/api/missing")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::error::Error::Api { code, message, detail }
                if code == "ORDER_NOT_FOUND" && message == "missing order" && detail == json!({"id": 1})
        ));
    }

    #[tokio::test]
    async fn a_error_body_is_mapped_through_http_client() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/account"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "code": "INVALID_REQUEST",
                "message": "bad request",
                "detail": {"field": "symbol"}
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new(ClientConfig::new(server.uri()).retry(0));
        let err = client
            .get_json::<serde_json::Value>("/v1/account")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::error::Error::Api { code, message, detail }
                if code == "INVALID_REQUEST" && message == "bad request" && detail == json!({"field": "symbol"})
        ));
    }
}
