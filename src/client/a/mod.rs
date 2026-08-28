//! A-share (同花顺) server client.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::config::{ClientConfig, encode_path_segment};
use crate::error::Result;
use crate::http::HttpClient;

pub mod types;

#[cfg(feature = "ws")]
pub mod ws;

pub use types::*;

/// Client for the A-share server.
///
/// The default base URL is `http://127.0.0.1:8787`.
#[derive(Debug, Clone)]
pub struct AClient {
    http: HttpClient,
}

impl AClient {
    /// Creates a client from a shared configuration.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            http: HttpClient::new(config),
        }
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &ClientConfig {
        self.http.config()
    }

    async fn a_get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.http.get_json(path).await
    }

    async fn a_get_with_query<T>(&self, path: &str, query: &[(&str, &str)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.http.get_json_with_query(path, query).await
    }

    async fn a_post<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.http.post_json(path, body).await
    }

    // ------------------------------------------------------------------
    // Read-only HTTP endpoints
    // ------------------------------------------------------------------

    /// Calls `GET /v1/account` and returns account funds with cache metadata.
    pub async fn account(&self) -> Result<Cached<AccountFunds>> {
        self.a_get("/v1/account").await
    }

    /// Calls `GET /v1/positions` and returns positions with cache metadata.
    pub async fn positions(&self) -> Result<Cached<Vec<Position>>> {
        self.a_get("/v1/positions").await
    }

    /// Calls `GET /v1/pnl` and returns P&L with cache metadata.
    pub async fn pnl(&self) -> Result<Cached<Pnl>> {
        self.a_get("/v1/pnl").await
    }

    /// Calls `GET /v1/account/transactions` and returns cash flows with cache
    /// metadata.
    pub async fn transactions(&self) -> Result<Cached<Vec<Transaction>>> {
        self.a_get("/v1/account/transactions").await
    }

    /// Calls `GET /v1/orders?type=order`.
    pub async fn list_orders(&self) -> Result<Cached<Vec<Order>>> {
        self.a_get_with_query("/v1/orders", &[("type", "order")])
            .await
    }

    /// Calls `GET /v1/orders?type=trade`.
    pub async fn list_trades(&self) -> Result<Cached<Vec<Trade>>> {
        self.a_get_with_query("/v1/orders", &[("type", "trade")])
            .await
    }

    /// Calls `GET /v1/orders?status=...`.
    pub async fn orders_by_status(&self, status: &str) -> Result<Cached<Vec<Order>>> {
        self.a_get_with_query("/v1/orders", &[("status", status)])
            .await
    }

    /// Calls `GET /v1/orders/{client_order_id}`.
    pub async fn get_order(&self, client_order_id: &str) -> Result<Cached<Order>> {
        let path = format!("/v1/orders/{}", encode_path_segment(client_order_id));
        self.a_get(&path).await
    }

    /// Calls `GET /v1/health` and returns the parsed health JSON.
    ///
    /// Kept from M1 for compatibility. Prefer [`Self::health_info`] when you
    /// want a typed result.
    pub async fn health(&self) -> Result<Value> {
        self.a_get("/v1/health").await
    }

    /// Calls `GET /v1/health` and returns a typed [`Health`] value.
    pub async fn health_info(&self) -> Result<Health> {
        self.a_get("/v1/health").await
    }

    /// Calls `GET /v1/metrics` and returns the raw Prometheus text.
    pub async fn metrics(&self) -> Result<String> {
        self.http.get_text("/v1/metrics").await
    }

    // ------------------------------------------------------------------
    // Write endpoints
    // ------------------------------------------------------------------

    /// Calls `POST /v1/refresh`.
    pub async fn refresh(&self) -> Result<RefreshResponse> {
        self.a_post("/v1/refresh", &serde_json::json!({})).await
    }

    /// Calls `POST /v1/notify/test`.
    pub async fn notify_test(&self) -> Result<NotifyTestResponse> {
        self.a_post("/v1/notify/test", &serde_json::json!({})).await
    }

    /// Calls `POST /v1/orders`.
    pub async fn submit_order(&self, request: &OrderRequest) -> Result<Order> {
        self.a_post("/v1/orders", request).await
    }

    /// Calls `POST /v1/orders/{client_order_id}/cancel`.
    ///
    /// `reason` is optional.
    pub async fn cancel_order(&self, client_order_id: &str, reason: Option<&str>) -> Result<Order> {
        let path = format!("/v1/orders/{}/cancel", encode_path_segment(client_order_id));
        let body = CancelRequest {
            reason: reason.map(str::to_owned),
        };
        self.a_post(&path, &body).await
    }

    /// Calls `POST /v1/orders/{client_order_id}/replace`.
    ///
    /// The request must set at least one of `new_price` / `new_quantity`.
    pub async fn replace_order(
        &self,
        client_order_id: &str,
        request: &ReplaceRequest,
    ) -> Result<Order> {
        request.validate()?;
        let path = format!(
            "/v1/orders/{}/replace",
            encode_path_segment(client_order_id)
        );
        self.a_post(&path, request).await
    }

    /// Calls `POST /v1/control/panic`.
    pub async fn panic(&self, reason: Option<&str>) -> Result<Value> {
        let body = PanicRequest {
            reason: reason.map(str::to_owned),
        };
        self.a_post("/v1/control/panic", &body).await
    }

    /// Calls `POST /v1/control/resume`.
    pub async fn resume(&self) -> Result<Value> {
        self.a_post("/v1/control/resume", &serde_json::json!({}))
            .await
    }

    // ------------------------------------------------------------------
    // WebSocket (feature `ws`)
    // ------------------------------------------------------------------

    /// Connects a single WebSocket stream.
    #[cfg(feature = "ws")]
    pub async fn connect_ws(&self) -> Result<ws::AWebSocket> {
        ws::connect(self).await
    }

    /// Returns an auto-reconnecting WebSocket event stream.
    #[cfg(feature = "ws")]
    pub async fn event_stream(&self) -> Result<ws::AEventStream> {
        ws::event_stream(self.clone())
    }
}

impl Default for AClient {
    fn default() -> Self {
        Self::new(ClientConfig::a_default())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn client(server: &MockServer) -> AClient {
        AClient::new(AClient::default().config().clone().base_url(server.uri()))
    }

    #[tokio::test]
    async fn default_base_url_matches_docs() {
        let client = AClient::default();
        assert_eq!(client.config().base_url, "http://127.0.0.1:8787");
    }

    #[tokio::test]
    async fn health_hits_v1_health_path() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
            .mount(&server)
            .await;

        let value = client(&server).health().await.unwrap();
        assert_eq!(value["status"], "ok");
    }

    #[tokio::test]
    async fn health_info_parses_typed_health() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "ok",
                "ths_online": true,
                "ax_permission": true,
                "gui_busy": false,
                "gui_queue_depth": 0,
                "last_refresh_at_ms": 1787910698040_i64,
                "panic": {
                    "panicked": false,
                    "reason": null,
                    "source": null,
                    "total_panics": 0
                },
                "audit_writable": true,
                "timestamp_ms": 1787911033806_i64
            })))
            .mount(&server)
            .await;

        let health = client(&server).health_info().await.unwrap();
        assert_eq!(health.status.as_deref(), Some("ok"));
        assert_eq!(health.ths_online, Some(true));
        assert_eq!(health.gui_busy, Some(false));
        assert_eq!(health.gui_queue_depth, Some(0.0));
        assert_eq!(health.last_refresh_at_ms, Some(1787910698040));
        assert_eq!(health.timestamp_ms, Some(1787911033806));
        assert!(health.panic.is_some());
    }

    #[tokio::test]
    async fn metrics_returns_raw_text() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_string("http_requests_total 1\n"))
            .mount(&server)
            .await;

        assert_eq!(
            client(&server).metrics().await.unwrap(),
            "http_requests_total 1\n"
        );
    }

    #[tokio::test]
    async fn account_returns_cached_account_funds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/account"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "account": "A1",
                "total_asset": 1000.0,
                "from_cache": true,
                "cached_at": "2026-08-28T10:00:00Z"
            })))
            .mount(&server)
            .await;

        let result = client(&server).account().await.unwrap();
        assert!(result.from_cache);
        assert_eq!(result.cached_at.as_deref(), Some("2026-08-28T10:00:00Z"));
        assert_eq!(result.data.total_asset, Some(1000.0));
    }

    #[tokio::test]
    async fn positions_pnl_and_transactions_hit_expected_paths() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/positions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"symbol": "512100", "quantity": 100}
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/pnl"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_pnl": 12.3
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/account/transactions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"transaction_id": "T1", "amount": 1.0}
            ])))
            .mount(&server)
            .await;

        let a = client(&server);
        let positions = a.positions().await.unwrap();
        assert_eq!(positions.data[0].symbol.as_deref(), Some("512100"));

        let pnl = a.pnl().await.unwrap();
        assert_eq!(pnl.data.total_pnl, Some(12.3));

        let transactions = a.transactions().await.unwrap();
        assert_eq!(transactions.data[0].transaction_id.as_deref(), Some("T1"));
    }

    #[tokio::test]
    async fn order_list_endpoints_use_documented_queries() {
        let server = MockServer::start().await;
        for (endpoint_path, query, body) in [
            (
                "/v1/orders",
                query_param("type", "order"),
                json!([{"client_order_id": "O1"}]),
            ),
            (
                "/v1/orders",
                query_param("type", "trade"),
                json!([{"trade_id": "T1"}]),
            ),
            (
                "/v1/orders",
                query_param("status", "Confirmed"),
                json!([{"status": "Confirmed"}]),
            ),
        ] {
            Mock::given(method("GET"))
                .and(path(endpoint_path))
                .and(query)
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&server)
                .await;
        }

        let a = client(&server);
        let orders = a.list_orders().await.unwrap();
        assert_eq!(orders.data[0].client_order_id.as_deref(), Some("O1"));

        let trades = a.list_trades().await.unwrap();
        assert_eq!(trades.data[0].trade_id.as_deref(), Some("T1"));

        let confirmed = a.orders_by_status("Confirmed").await.unwrap();
        assert_eq!(confirmed.data[0].status.as_deref(), Some("Confirmed"));
    }

    #[tokio::test]
    async fn get_order_hits_client_order_id_path() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/orders/C001"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "client_order_id": "C001",
                "status": "FILLED"
            })))
            .mount(&server)
            .await;

        let order = client(&server).get_order("C001").await.unwrap();
        assert_eq!(order.data.client_order_id.as_deref(), Some("C001"));
    }

    #[tokio::test]
    async fn get_order_url_encodes_special_client_order_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/orders/a%2Fb%3Fc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "client_order_id": "a/b?c",
                "status": "FILLED"
            })))
            .mount(&server)
            .await;

        let order = client(&server).get_order("a/b?c").await.unwrap();
        assert_eq!(order.data.client_order_id.as_deref(), Some("a/b?c"));
    }

    #[tokio::test]
    async fn refresh_posts_v1_refresh() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "ok",
                "message": "refreshed"
            })))
            .mount(&server)
            .await;

        let result = client(&server).refresh().await.unwrap();
        assert_eq!(result.status.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn notify_test_posts_v1_notify_test() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/notify/test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ok": true,
                "title": "自动熔断",
                "message": "测试报警"
            })))
            .mount(&server)
            .await;

        let result = client(&server).notify_test().await.unwrap();
        assert!(result.ok);
        assert_eq!(result.title.as_deref(), Some("自动熔断"));
        assert_eq!(result.message.as_deref(), Some("测试报警"));
    }

    #[tokio::test]
    async fn submit_order_sends_documented_body_for_dry_run_and_real() {
        let server = MockServer::start().await;
        for (dry_run, expected) in [(false, false), (true, true)] {
            Mock::given(method("POST"))
                .and(path("/v1/orders"))
                .and(body_json(json!({
                    "client_order_id": "C001",
                    "symbol": "512100",
                    "side": "buy",
                    "price": 3.305,
                    "quantity": 100,
                    "dry_run": dry_run
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "client_order_id": "C001",
                    "status": "SUBMITTED",
                    "dry_run": dry_run
                })))
                .mount(&server)
                .await;
            let request = OrderRequest::new("C001", "512100", "buy", 3.305, 100, dry_run);
            let order = client(&server).submit_order(&request).await.unwrap();
            assert_eq!(order.status.as_deref(), Some("SUBMITTED"));
            assert_eq!(order.dry_run, Some(expected));
        }
    }

    #[tokio::test]
    async fn cancel_order_sends_optional_reason() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/orders/C001/cancel"))
            .and(body_json(json!({"reason": "manual"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "CANCELLED"})))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/orders/C002/cancel"))
            .and(body_json(json!({})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "CANCELLED"})))
            .mount(&server)
            .await;

        let a = client(&server);
        let with_reason = a.cancel_order("C001", Some("manual")).await.unwrap();
        assert_eq!(with_reason.status.as_deref(), Some("CANCELLED"));
        let without_reason = a.cancel_order("C002", None).await.unwrap();
        assert_eq!(without_reason.status.as_deref(), Some("CANCELLED"));
    }

    #[tokio::test]
    async fn replace_order_sends_documented_body_and_validates() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/orders/C001/replace"))
            .and(body_json(json!({
                "action": "replace",
                "order_no": "12345",
                "new_price": 3.30,
                "new_quantity": 200,
                "dry_run": false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "REPLACED"
            })))
            .mount(&server)
            .await;

        let request = ReplaceRequest::builder("12345", false)
            .with_price(3.30)
            .with_quantity(200);
        let result = client(&server)
            .replace_order("C001", &request)
            .await
            .unwrap();
        assert_eq!(result.status.as_deref(), Some("REPLACED"));

        let invalid = ReplaceRequest::builder("12345", false);
        assert!(
            client(&server)
                .replace_order("C001", &invalid)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn panic_and_resume_hit_control_paths() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/control/panic"))
            .and(body_json(json!({"reason": "发现异常"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"panic": true})))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/control/resume"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"panic": false})))
            .mount(&server)
            .await;

        let a = client(&server);
        let panic = a.panic(Some("发现异常")).await.unwrap();
        assert_eq!(panic["panic"], true);
        let resume = a.resume().await.unwrap();
        assert_eq!(resume["panic"], false);
    }

    #[tokio::test]
    async fn every_a_http_endpoint_maps_a_error_body() {
        let server = MockServer::start().await;
        // All documented HTTP endpoints. The shared HTTP layer maps the
        // `{code,message,detail}` body for every one of them.
        for (method_name, endpoint_path) in [
            ("GET", "/v1/account"),
            ("GET", "/v1/positions"),
            ("GET", "/v1/orders"),
            ("GET", "/v1/orders/C001"),
            ("GET", "/v1/pnl"),
            ("GET", "/v1/account/transactions"),
            ("GET", "/v1/health"),
            ("GET", "/v1/metrics"),
            ("POST", "/v1/refresh"),
            ("POST", "/v1/notify/test"),
            ("POST", "/v1/orders"),
            ("POST", "/v1/orders/C001/cancel"),
            ("POST", "/v1/orders/C001/replace"),
            ("POST", "/v1/control/panic"),
            ("POST", "/v1/control/resume"),
        ] {
            Mock::given(method(method_name))
                .and(path(endpoint_path))
                .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                    "code": "error",
                    "message": "bad request",
                    "detail": {"field": "x"}
                })))
                .mount(&server)
                .await;
        }

        let a = client(&server);
        assert!(a.account().await.is_err());
        assert!(a.positions().await.is_err());
        assert!(a.list_orders().await.is_err());
        assert!(a.list_trades().await.is_err());
        assert!(a.orders_by_status("Confirmed").await.is_err());
        assert!(a.get_order("C001").await.is_err());
        assert!(a.pnl().await.is_err());
        assert!(a.transactions().await.is_err());
        assert!(a.health().await.is_err());
        assert!(a.health_info().await.is_err());
        assert!(a.metrics().await.is_err());
        assert!(a.refresh().await.is_err());
        assert!(a.notify_test().await.is_err());
        assert!(
            a.submit_order(&OrderRequest::new("C", "S", "buy", 1.0, 1, false))
                .await
                .is_err()
        );
        assert!(a.cancel_order("C001", None).await.is_err());
        assert!(
            a.replace_order("C001", &ReplaceRequest::price("H1", 3.3, false))
                .await
                .is_err()
        );
        assert!(a.panic(None).await.is_err());
        assert!(a.resume().await.is_err());
    }
}
