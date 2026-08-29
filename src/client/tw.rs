//! Taiwan stock-broker client.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::config::{ClientConfig, encode_path_segment};
use crate::error::Result;
use crate::http::HttpClient;
use crate::response::TwEnvelope;

pub mod types;

#[cfg(feature = "ws")]
pub mod ws;

pub use types::*;

/// Client for the stock-broker-tw-server.
///
/// The default base URL is `http://127.0.0.1:8000`.
#[derive(Debug, Clone)]
pub struct TwClient {
    http: HttpClient,
    subscriptions: Arc<Mutex<Vec<QuoteSubscription>>>,
}

impl TwClient {
    /// Creates a client with the default TW server configuration.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            http: HttpClient::new(config),
            subscriptions: Arc::new(Mutex::new(Vec::new())),
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

    /// Calls `GET /health` and returns a typed [`Health`] value.
    pub async fn health_info(&self) -> Result<Health> {
        self.http.get_json("/health").await
    }

    /// Calls `GET /metrics` and returns the raw Prometheus text.
    pub async fn metrics(&self) -> Result<String> {
        self.http.get_text("/metrics").await
    }

    async fn tw_get<T>(&self, path: &str, query: &[(&str, &str)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let envelope: TwEnvelope<T> = self.http.get_json_with_query(path, query).await?;
        envelope.into_data()
    }

    async fn tw_post<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let envelope: TwEnvelope<T> = self.http.post_json(path, body).await?;
        envelope.into_data()
    }

    async fn tw_post_unit<B>(&self, path: &str, body: Option<&B>) -> Result<()>
    where
        B: Serialize,
    {
        let envelope: TwEnvelope<Value> = if let Some(body) = body {
            self.http.post_json(path, body).await?
        } else {
            self.http.post_json(path, &serde_json::json!({})).await?
        };
        envelope.into_unit()
    }

    // ------------------------------------------------------------------
    // Session
    // ------------------------------------------------------------------

    /// Logs in. Pass `None` to use the server-default account/credentials.
    pub async fn login(&self, request: Option<LoginRequest>) -> Result<LoginResponse> {
        let body = request.unwrap_or_default();
        self.tw_post("/api/v1/session/login", &body).await
    }

    /// Logs out.
    pub async fn logout(&self) -> Result<()> {
        self.tw_post_unit::<Value>("/api/v1/session/logout", None)
            .await
    }

    /// Queries the current session status.
    pub async fn session_status(&self) -> Result<SessionStatus> {
        self.tw_get("/api/v1/session/status", &[]).await
    }

    // ------------------------------------------------------------------
    // Account and queries
    // ------------------------------------------------------------------

    /// Returns positions. `account` is optional.
    pub async fn positions(&self, account: Option<&str>) -> Result<Vec<Position>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        self.tw_get("/api/v1/positions", &query).await
    }

    /// Returns the account bank balance. `account` is optional.
    pub async fn balance(&self, account: Option<&str>) -> Result<Balance> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        self.tw_get("/api/v1/account/balance", &query).await
    }

    /// Returns settlement amounts. `account` is optional.
    pub async fn settlement(&self, account: Option<&str>) -> Result<Vec<Settlement>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        self.tw_get("/api/v1/account/settlement", &query).await
    }

    /// Returns unrealized P&L. `account` is optional.
    pub async fn pnl_unrealized(&self, account: Option<&str>) -> Result<Vec<PnlUnrealized>> {
        self.pnl_unrealized_with(account, None, None).await
    }

    /// Returns unrealized P&L with optional account, market type and stock code filters.
    pub async fn pnl_unrealized_with(
        &self,
        account: Option<&str>,
        market_type: Option<&str>,
        stk_code: Option<&str>,
    ) -> Result<Vec<PnlUnrealized>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        if let Some(market_type) = market_type {
            query.push(("market_type", market_type));
        }
        if let Some(stk_code) = stk_code {
            query.push(("stk_code", stk_code));
        }
        self.tw_get("/api/v1/pnl/unrealized", &query).await
    }

    /// Returns realized P&L. Supports optional `account`, `start_date`, and `end_date`.
    pub async fn pnl_realized(
        &self,
        account: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<PnlRealized>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        if let Some(start_date) = start_date {
            query.push(("start_date", start_date));
        }
        if let Some(end_date) = end_date {
            query.push(("end_date", end_date));
        }
        self.tw_get("/api/v1/pnl/realized", &query).await
    }

    /// Returns reversal P&L. `account` is optional.
    pub async fn pnl_reversal(&self, account: Option<&str>) -> Result<Vec<PnlReversal>> {
        self.pnl_reversal_with(account, None).await
    }

    /// Returns reversal P&L with an optional account and `re_gain_loss` query value.
    pub async fn pnl_reversal_with(
        &self,
        account: Option<&str>,
        re_gain_loss: Option<&str>,
    ) -> Result<Vec<PnlReversal>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        if let Some(re_gain_loss) = re_gain_loss {
            query.push(("re_gain_loss", re_gain_loss));
        }
        self.tw_get("/api/v1/pnl/reversal", &query).await
    }

    /// Returns real-time reports. `account` is optional.
    pub async fn reports_real(&self, account: Option<&str>) -> Result<Vec<RealReport>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        self.tw_get("/api/v1/reports/real", &query).await
    }

    /// Returns merged real-time reports. `account` is optional.
    pub async fn reports_real_merge(&self, account: Option<&str>) -> Result<Vec<RealReportMerge>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        self.tw_get("/api/v1/reports/real-merge", &query).await
    }

    /// Returns order/trade reports. `account` and `notshow_cancel` are optional.
    pub async fn reports_order_trade(
        &self,
        account: Option<&str>,
        notshow_cancel: Option<bool>,
    ) -> Result<Vec<OrderTradeReport>> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(account) = account {
            query.push(("account", account.to_owned()));
        }
        if let Some(notshow_cancel) = notshow_cancel {
            query.push(("notshow_cancel", notshow_cancel.to_string()));
        }
        let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();
        self.tw_get("/api/v1/reports/order-trade", &query).await
    }

    // ------------------------------------------------------------------
    // Quotes
    // ------------------------------------------------------------------

    /// Subscribes to quotes and remembers the subscription for WebSocket resubscribe.
    pub async fn subscribe_quotes(&self, request: &QuoteSubscription) -> Result<()> {
        self.subscribe_quotes_http(request).await?;
        let mut subscriptions = self
            .subscriptions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !subscriptions.contains(request) {
            subscriptions.push(request.clone());
        }
        Ok(())
    }

    /// Calls the HTTP subscribe endpoint without mutating the remembered list.
    ///
    /// This is used by the WebSocket reconnect loop to restore subscriptions
    /// through the documented HTTP API.
    pub(crate) async fn subscribe_quotes_http(&self, request: &QuoteSubscription) -> Result<()> {
        self.tw_post_unit("/api/v1/quotes/subscribe", Some(request))
            .await
    }

    /// Unsubscribes from quotes and removes the remembered subscription.
    pub async fn unsubscribe_quotes(&self, request: &QuoteSubscription) -> Result<()> {
        self.tw_post_unit("/api/v1/quotes/unsubscribe", Some(request))
            .await?;
        let mut subscriptions = self
            .subscriptions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        subscriptions.retain(|existing| existing != request);
        Ok(())
    }

    /// Returns currently subscribed quotes from the selected source.
    pub async fn subscribed_quotes(
        &self,
        source: SubscribedSource,
    ) -> Result<Vec<QuoteSubscription>> {
        let source = match source {
            SubscribedSource::Local => "local",
            SubscribedSource::Broker => "broker",
            SubscribedSource::Both => "both",
        };
        let query = [("source", source)];
        self.tw_get("/api/v1/quotes/subscribed", &query).await
    }

    /// Returns a quote snapshot.
    pub async fn quote_snapshot(
        &self,
        stk_code: &str,
        market_type: Option<&str>,
    ) -> Result<QuoteSnapshot> {
        let mut query = vec![("stk_code", stk_code)];
        if let Some(market_type) = market_type {
            query.push(("market_type", market_type));
        }
        self.tw_get("/api/v1/quotes/snapshot", &query).await
    }

    /// Returns intraday ticks.
    pub async fn quote_ticks(
        &self,
        stk_code: &str,
        market_type: Option<&str>,
        last_count: Option<u32>,
    ) -> Result<Vec<Tick>> {
        let mut query: Vec<(&str, String)> = vec![("stk_code", stk_code.to_owned())];
        if let Some(market_type) = market_type {
            query.push(("market_type", market_type.to_owned()));
        }
        if let Some(last_count) = last_count {
            query.push(("last_count", last_count.to_string()));
        }
        let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();
        self.tw_get("/api/v1/quotes/ticks", &query).await
    }

    /// Returns classify-price rows.
    pub async fn quote_classify_price(
        &self,
        stk_code: &str,
        market_type: Option<&str>,
    ) -> Result<Vec<ClassifyPrice>> {
        let mut query = vec![("stk_code", stk_code)];
        if let Some(market_type) = market_type {
            query.push(("market_type", market_type));
        }
        self.tw_get("/api/v1/quotes/classify-price", &query).await
    }

    /// Returns K-line bars.
    pub async fn quote_kline(
        &self,
        stk_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<Kline>> {
        let mut query = vec![("stk_code", stk_code)];
        if let Some(start_date) = start_date {
            query.push(("start_date", start_date));
        }
        if let Some(end_date) = end_date {
            query.push(("end_date", end_date));
        }
        self.tw_get("/api/v1/quotes/kline", &query).await
    }

    /// Returns stock information.
    pub async fn stock_info(&self, stk_code: &str, market_type: Option<&str>) -> Result<StockInfo> {
        let mut query = vec![("stk_code", stk_code)];
        if let Some(market_type) = market_type {
            query.push(("market_type", market_type));
        }
        self.tw_get("/api/v1/stocks/info", &query).await
    }

    // ------------------------------------------------------------------
    // Trading
    // ------------------------------------------------------------------

    /// Sends any stock order request to the unified order endpoint.
    ///
    /// This is the low-level entry point; the convenience methods
    /// [`TwClient::cancel_order`], [`TwClient::replace_price`] and
    /// [`TwClient::replace_quantity`] build safe requests.
    pub async fn submit_stock_order(&self, request: &OrderRequest) -> Result<OrderStatus> {
        self.tw_post("/api/v1/orders/stock", request).await
    }

    /// Cancels an order.
    #[allow(clippy::too_many_arguments)]
    pub async fn cancel_order(
        &self,
        client_order_id: &str,
        account: &str,
        order_no: &str,
        trade_date: &str,
        stk_code: &str,
        side: &str,
    ) -> Result<OrderStatus> {
        let request = OrderRequest::cancel(
            client_order_id,
            account,
            order_no,
            trade_date,
            stk_code,
            side,
        );
        self.submit_stock_order(&request).await
    }

    /// Replaces an order price.
    #[allow(clippy::too_many_arguments)]
    pub async fn replace_price(
        &self,
        client_order_id: &str,
        account: &str,
        order_no: &str,
        stk_code: &str,
        side: &str,
        new_price: f64,
    ) -> Result<OrderStatus> {
        let request = OrderRequest::replace_price(
            client_order_id,
            account,
            order_no,
            stk_code,
            side,
            new_price,
        );
        self.submit_stock_order(&request).await
    }

    /// Replaces an order quantity.
    #[allow(clippy::too_many_arguments)]
    pub async fn replace_quantity(
        &self,
        client_order_id: &str,
        account: &str,
        order_no: &str,
        stk_code: &str,
        side: &str,
        new_quantity: i64,
    ) -> Result<OrderStatus> {
        let request = OrderRequest::replace_quantity(
            client_order_id,
            account,
            order_no,
            stk_code,
            side,
            new_quantity,
        );
        self.submit_stock_order(&request).await
    }

    /// Lists orders, optionally filtered by account and status.
    pub async fn list_orders(
        &self,
        account: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<OrderRecord>> {
        let mut query = Vec::new();
        if let Some(account) = account {
            query.push(("account", account));
        }
        if let Some(status) = status {
            query.push(("status", status));
        }
        self.tw_get("/api/v1/orders", &query).await
    }

    /// Gets one order by `client_order_id`.
    pub async fn get_order(&self, client_order_id: &str) -> Result<OrderRecord> {
        let path = format!("/api/v1/orders/{}", encode_path_segment(client_order_id));
        self.tw_get(&path, &[]).await
    }

    // ------------------------------------------------------------------
    // Risk control
    // ------------------------------------------------------------------

    /// Enables manual panic mode.
    pub async fn panic(&self) -> Result<()> {
        self.tw_post_unit::<Value>("/api/v1/control/panic", None)
            .await
    }

    /// Disables panic mode and resets the circuit breaker.
    pub async fn resume(&self) -> Result<()> {
        self.tw_post_unit::<Value>("/api/v1/control/resume", None)
            .await
    }

    // ------------------------------------------------------------------
    // Recovery
    // ------------------------------------------------------------------

    /// Returns unresolved recovery items.
    pub async fn recovery_unresolved(&self) -> Result<Vec<RecoveryItem>> {
        self.tw_get("/api/v1/recovery/unresolved", &[]).await
    }

    /// Resolves a recovery item manually.
    pub async fn recovery_resolve(
        &self,
        client_order_id: &str,
        request: &RecoveryResolveRequest,
    ) -> Result<RecoveryItem> {
        let path = format!(
            "/api/v1/recovery/{}/resolve",
            encode_path_segment(client_order_id)
        );
        self.tw_post(&path, request).await
    }

    // ------------------------------------------------------------------
    // WebSocket (feature `ws`)
    // ------------------------------------------------------------------

    /// Connects a single WebSocket stream.
    #[cfg(feature = "ws")]
    pub async fn connect_ws(&self) -> Result<ws::TwWebSocket> {
        ws::connect(self).await
    }

    /// Returns an auto-reconnecting WebSocket event stream.
    #[cfg(feature = "ws")]
    pub async fn event_stream(&self) -> Result<ws::TwEventStream> {
        ws::event_stream(self.clone())
    }
}

impl Default for TwClient {
    fn default() -> Self {
        Self::new(ClientConfig::tw_default())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{any, body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

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
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"ok"})))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let value = client.health().await.unwrap();
        assert_eq!(value["status"], "ok");
    }

    #[tokio::test]
    async fn health_info_parses_typed_health() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "ok",
                "adapter_ready": true,
                "login_status": true,
                "panic": false
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let health = client.health_info().await.unwrap();
        assert_eq!(health.status, "ok");
        assert!(health.adapter_ready);
        assert!(health.login_status);
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

    #[tokio::test]
    async fn login_sends_empty_body_by_default_and_parses_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/session/login"))
            .and(body_json(json!({})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {
                    "login": {"login_list": []},
                    "account": "S98875005091",
                    "name": "測試",
                    "investor_id": "A123"
                }
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let resp = client.login(None).await.unwrap();
        assert_eq!(resp.account.as_deref(), Some("S98875005091"));
    }

    #[tokio::test]
    async fn positions_hits_path_with_account_query() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/positions"))
            .and(query_param("account", "S1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": [{"account": "S1", "stk_code": "2330", "quantity": 10}]
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let positions = client.positions(Some("S1")).await.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].stk_code.as_deref(), Some("2330"));
    }

    #[tokio::test]
    async fn submit_order_sends_documented_new_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/orders/stock"))
            .and(body_json(json!({
                "client_order_id": "C001",
                "action": "new",
                "account": "S98875005091",
                "stk_code": "2330",
                "side": "B",
                "price": 500.0,
                "quantity": 10,
                "time_in_force": "ROD",
                "price_flag": "LIMIT"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {"client_order_id": "C001", "status": "SUBMITTED"}
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let request = OrderRequest::new(
            "C001",
            "S98875005091",
            "2330",
            "B",
            500.0,
            10,
            "ROD",
            "LIMIT",
        );
        let status = client.submit_stock_order(&request).await.unwrap();
        assert_eq!(status.status.as_deref(), Some("SUBMITTED"));
    }

    #[tokio::test]
    async fn replace_price_does_not_include_new_quantity() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/orders/stock"))
            .and(body_json(json!({
                "client_order_id": "C003",
                "action": "replace",
                "account": "S98875005091",
                "order_no": "H00001",
                "stk_code": "2330",
                "side": "B",
                "new_price": 510.0
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {"client_order_id": "C003", "status": "ACCEPTED"}
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let status = client
            .replace_price("C003", "S98875005091", "H00001", "2330", "B", 510.0)
            .await
            .unwrap();
        assert_eq!(status.status.as_deref(), Some("ACCEPTED"));
    }

    #[tokio::test]
    async fn api_error_envelope_is_mapped() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/orders/C001"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "detail": {
                    "code": "ORDER_NOT_FOUND",
                    "message": "missing",
                    "detail": {}
                }
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let err = client.get_order("C001").await.unwrap_err();
        assert!(matches!(err, crate::error::Error::Api { .. }));
    }

    #[tokio::test]
    async fn logout_panic_and_resume_accept_missing_data() {
        let server = MockServer::start().await;
        for (http_method, endpoint_path) in [
            ("POST", "/api/v1/session/logout"),
            ("POST", "/api/v1/control/panic"),
            ("POST", "/api/v1/control/resume"),
        ] {
            Mock::given(method(http_method))
                .and(path(endpoint_path))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "code": 0,
                    "message": "ok"
                })))
                .mount(&server)
                .await;
        }

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        client.logout().await.unwrap();
        client.panic().await.unwrap();
        client.resume().await.unwrap();
    }

    #[tokio::test]
    async fn session_and_account_query_methods_hit_expected_paths() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/session/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"logged_in": true, "account": "S1"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/account/balance"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"account": "S1", "total_balance": 100.0}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/account/settlement"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"account": "S1", "amount": 50.0}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pnl/unrealized"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"stk_code": "2330", "pnl": 1.0}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pnl/realized"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"stk_code": "2330", "realized_pnl": 2.0}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pnl/reversal"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"stk_code": "2330", "amount": 3.0}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/reports/real"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"order_no": "H1"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/reports/real-merge"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"order_no": "H1"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/reports/order-trade"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"order_no": "H1", "status": "FILLED"}]
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let session = client.session_status().await.unwrap();
        assert!(session.logged_in);
        assert_eq!(
            client.balance(None).await.unwrap().total_balance,
            Some(100.0)
        );
        assert_eq!(client.settlement(None).await.unwrap().len(), 1);
        assert_eq!(client.pnl_unrealized(None).await.unwrap().len(), 1);
        assert_eq!(
            client.pnl_realized(None, None, None).await.unwrap().len(),
            1
        );
        assert_eq!(client.pnl_reversal(None).await.unwrap().len(), 1);
        assert_eq!(client.reports_real(None).await.unwrap().len(), 1);
        assert_eq!(client.reports_real_merge(None).await.unwrap().len(), 1);
        assert_eq!(
            client.reports_order_trade(None, None).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn quote_subscribe_unsubscribe_and_queries_work() {
        let server = MockServer::start().await;
        let sub = QuoteSubscription {
            r#type: QuoteType::FiveTick,
            symbols: vec!["2330".to_owned()],
            account: Some("S1".to_owned()),
            market_type: Some("TWSE".to_owned()),
            index_flag: Some(7),
        };

        Mock::given(method("POST"))
            .and(path("/api/v1/quotes/subscribe"))
            .and(body_json(serde_json::to_value(&sub).unwrap()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"code": 0, "message": "ok"})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/quotes/unsubscribe"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"code": 0, "message": "ok"})),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/quotes/subscribed"))
            .and(query_param("source", "both"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"type": "five_tick", "symbols": ["2330"]}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/quotes/snapshot"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"stk_code": "2330", "last_price": 500.0}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/quotes/ticks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"time": "09:00", "price": 500.0}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/quotes/classify-price"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"price": 500.0, "volume": 10}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/quotes/kline"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"date": "2026/01/01", "close": 500.0}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/stocks/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"stk_code": "2330", "name": "台積電"}
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        client.subscribe_quotes(&sub).await.unwrap();
        client.unsubscribe_quotes(&sub).await.unwrap();
        assert_eq!(
            client
                .subscribed_quotes(SubscribedSource::Both)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .quote_snapshot("2330", Some("TWSE"))
                .await
                .unwrap()
                .stk_code
                .as_deref(),
            Some("2330")
        );
        assert_eq!(
            client
                .quote_ticks("2330", Some("TWSE"), Some(20))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .quote_classify_price("2330", Some("TWSE"))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .quote_kline("2330", Some("2026/01/01"), Some("2026/01/31"))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .stock_info("2330", Some("TWSE"))
                .await
                .unwrap()
                .name
                .as_deref(),
            Some("台積電")
        );
    }

    #[tokio::test]
    async fn cancel_replace_quantity_list_and_get_order_work() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/orders/stock"))
            .and(body_json(json!({
                "client_order_id": "C002",
                "action": "cancel",
                "account": "S1",
                "order_no": "H1",
                "trade_date": "2026/08/28",
                "stk_code": "2330",
                "side": "B"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"client_order_id": "C002", "status": "CANCELLED"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/orders/stock"))
            .and(body_json(json!({
                "client_order_id": "C004",
                "action": "replace",
                "account": "S1",
                "order_no": "H1",
                "stk_code": "2330",
                "side": "B",
                "new_quantity": 20
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"client_order_id": "C004", "status": "ACCEPTED"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/orders"))
            .and(query_param("status", "ACCEPTED"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"client_order_id": "C1", "status": "ACCEPTED"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/orders/C1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"client_order_id": "C1", "status": "ACCEPTED"}
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        assert_eq!(
            client
                .cancel_order("C002", "S1", "H1", "2026/08/28", "2330", "B")
                .await
                .unwrap()
                .status
                .as_deref(),
            Some("CANCELLED")
        );
        assert_eq!(
            client
                .replace_quantity("C004", "S1", "H1", "2330", "B", 20)
                .await
                .unwrap()
                .status
                .as_deref(),
            Some("ACCEPTED")
        );
        assert_eq!(
            client
                .list_orders(None, Some("ACCEPTED"))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .get_order("C1")
                .await
                .unwrap()
                .client_order_id
                .as_deref(),
            Some("C1")
        );
    }

    #[tokio::test]
    async fn get_order_url_encodes_special_client_order_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/orders/a%2Fb%3Fc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {"client_order_id": "a/b?c", "status": "FILLED"}
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let order = client.get_order("a/b?c").await.unwrap();
        assert_eq!(order.client_order_id.as_deref(), Some("a/b?c"));
    }

    #[tokio::test]
    async fn recovery_endpoints_work() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/recovery/unresolved"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": [{"client_order_id": "C1", "source": "stock_orders"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/recovery/C1/resolve"))
            .and(body_json(json!({
                "status": "FILLED",
                "source": "stock_orders",
                "note": "ok"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0, "message": "ok",
                "data": {"client_order_id": "C1", "status": "FILLED"}
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        assert_eq!(client.recovery_unresolved().await.unwrap().len(), 1);
        let req = RecoveryResolveRequest::new("FILLED")
            .source("stock_orders")
            .note("ok");
        let item = client.recovery_resolve("C1", &req).await.unwrap();
        assert_eq!(item.status.as_deref(), Some("FILLED"));
    }

    #[tokio::test]
    async fn nonzero_tw_code_with_http_200_maps_to_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/positions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 123,
                "message": "bad",
                "data": null
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let err = client.positions(None).await.unwrap_err();
        assert!(matches!(err, crate::error::Error::Api { code, .. } if code == "123"));
    }

    #[tokio::test]
    async fn main_flow_login_positions_submit_get_cancel_logout() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/session/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {
                    "login": {"login_list": []},
                    "account": "S1",
                    "name": "測試",
                    "investor_id": "A1"
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/positions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": [{"stk_code": "2330", "quantity": 10}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/orders/stock"))
            .and(body_json(json!({
                "client_order_id": "C001",
                "action": "new",
                "account": "S1",
                "stk_code": "2330",
                "side": "B",
                "price": 500.0,
                "quantity": 10,
                "time_in_force": "ROD",
                "price_flag": "LIMIT"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {"client_order_id": "C001", "status": "ACCEPTED"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/orders/C001"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {"client_order_id": "C001", "status": "ACCEPTED"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/orders/stock"))
            .and(body_json(json!({
                "client_order_id": "C001",
                "action": "cancel",
                "account": "S1",
                "order_no": "H00001",
                "trade_date": "2026/08/28",
                "stk_code": "2330",
                "side": "B"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "data": {"client_order_id": "C001", "status": "CANCELLED"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/session/logout"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"code": 0, "message": "ok"})),
            )
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let login = client.login(None).await.unwrap();
        assert_eq!(login.account.as_deref(), Some("S1"));
        assert_eq!(client.positions(None).await.unwrap().len(), 1);

        let order = OrderRequest::new("C001", "S1", "2330", "B", 500.0, 10, "ROD", "LIMIT");
        let submitted = client.submit_stock_order(&order).await.unwrap();
        assert_eq!(submitted.status.as_deref(), Some("ACCEPTED"));
        assert_eq!(
            client
                .get_order("C001")
                .await
                .unwrap()
                .client_order_id
                .as_deref(),
            Some("C001")
        );

        let cancelled = client
            .cancel_order("C001", "S1", "H00001", "2026/08/28", "2330", "B")
            .await
            .unwrap();
        assert_eq!(cancelled.status.as_deref(), Some("CANCELLED"));
        client.logout().await.unwrap();
    }

    #[tokio::test]
    async fn every_tw_http_method_maps_tw_error_envelope() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "detail": {
                    "code": "TW_ERROR",
                    "message": "boom",
                    "detail": {}
                }
            })))
            .mount(&server)
            .await;

        let client = TwClient::new(TwClient::default().config().clone().base_url(server.uri()));
        let sub = QuoteSubscription::new(QuoteType::FiveTick, vec!["2330".to_owned()]);
        let order = OrderRequest::new("C1", "S1", "2330", "B", 500.0, 10, "ROD", "LIMIT");

        assert!(client.login(None).await.is_err());
        assert!(client.logout().await.is_err());
        assert!(client.session_status().await.is_err());
        assert!(client.positions(None).await.is_err());
        assert!(client.balance(None).await.is_err());
        assert!(client.settlement(None).await.is_err());
        assert!(client.pnl_unrealized(None).await.is_err());
        assert!(client.pnl_realized(None, None, None).await.is_err());
        assert!(client.pnl_reversal(None).await.is_err());
        assert!(client.reports_real(None).await.is_err());
        assert!(client.reports_real_merge(None).await.is_err());
        assert!(client.reports_order_trade(None, None).await.is_err());
        assert!(client.subscribe_quotes(&sub).await.is_err());
        assert!(client.unsubscribe_quotes(&sub).await.is_err());
        assert!(
            client
                .subscribed_quotes(SubscribedSource::Both)
                .await
                .is_err()
        );
        assert!(client.quote_snapshot("2330", None).await.is_err());
        assert!(client.quote_ticks("2330", None, None).await.is_err());
        assert!(client.quote_classify_price("2330", None).await.is_err());
        assert!(client.quote_kline("2330", None, None).await.is_err());
        assert!(client.stock_info("2330", None).await.is_err());
        assert!(client.submit_stock_order(&order).await.is_err());
        assert!(
            client
                .cancel_order("C1", "S1", "H1", "2026/08/28", "2330", "B")
                .await
                .is_err()
        );
        assert!(
            client
                .replace_price("C1", "S1", "H1", "2330", "B", 510.0)
                .await
                .is_err()
        );
        assert!(
            client
                .replace_quantity("C1", "S1", "H1", "2330", "B", 20)
                .await
                .is_err()
        );
        assert!(client.list_orders(None, None).await.is_err());
        assert!(client.get_order("C1").await.is_err());
        assert!(client.panic().await.is_err());
        assert!(client.resume().await.is_err());
        assert!(client.recovery_unresolved().await.is_err());
        assert!(
            client
                .recovery_resolve("C1", &RecoveryResolveRequest::new("FILLED"))
                .await
                .is_err()
        );
    }
}
