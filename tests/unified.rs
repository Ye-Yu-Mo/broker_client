//! Integration tests for the unified `BrokerClient` trait.

use broker_client::{
    AClient, BrokerClient, CancelOrderRequest, ClientConfig, OrderAction, OrderRequest, TwClient,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn a_client(server: &MockServer) -> AClient {
    AClient::new(ClientConfig::new(server.uri()))
}

fn tw_client(server: &MockServer) -> TwClient {
    TwClient::new(ClientConfig::new(server.uri()))
}

fn unified_new_order() -> OrderRequest {
    OrderRequest {
        client_order_id: "C1".to_owned(),
        action: OrderAction::New,
        account: "S1".to_owned(),
        stk_code: "2330".to_owned(),
        symbol: Some("2330".to_owned()),
        side: Some("B".to_owned()),
        price: Some(500.0),
        quantity: Some(10),
        time_in_force: Some("ROD".to_owned()),
        price_flag: Some("LIMIT".to_owned()),
        order_no: None,
        trade_date: None,
        new_price: None,
        new_quantity: None,
        dry_run: Some(false),
    }
}

fn unified_cancel() -> CancelOrderRequest {
    CancelOrderRequest::tw("C1", "S1", "H1", "2026/08/28", "2330", "B")
}

async fn run_unified_flow(client: &dyn BrokerClient) {
    let health = client.health().await.unwrap();
    assert!(health.status.is_some());

    let account = client.account().await.unwrap();
    assert!(account.account.is_some());

    let positions = client.positions().await.unwrap();
    assert!(!positions.is_empty());
    assert!(positions[0].symbol.is_some() || positions[0].stk_code.is_some());

    let submitted = client.submit_order(&unified_new_order()).await.unwrap();
    assert_eq!(submitted.client_order_id.as_deref(), Some("C1"));

    let fetched = client.get_order("C1").await.unwrap();
    assert_eq!(fetched.client_order_id.as_deref(), Some("C1"));

    let cancelled = client.cancel_order(&unified_cancel()).await.unwrap();
    assert_eq!(cancelled.client_order_id.as_deref(), Some("C1"));
}

#[tokio::test]
async fn a_client_implements_unified_trait_flow() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "ok",
            "ths_online": true
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account": "A1",
            "total_asset": 1000.0
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/positions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "symbol": "512100",
            "name": "沪深300ETF",
            "quantity": 100
        }])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/orders"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "client_order_id": "C1",
            "symbol": "2330",
            "status": "SUBMITTED"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/orders/C1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "client_order_id": "C1",
            "symbol": "2330",
            "status": "SUBMITTED"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/orders/C1/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "client_order_id": "C1",
            "symbol": "2330",
            "status": "CANCELLED"
        })))
        .mount(&server)
        .await;

    let client: Box<dyn BrokerClient> = Box::new(a_client(&server));
    run_unified_flow(client.as_ref()).await;
}

#[tokio::test]
async fn tw_client_implements_unified_trait_flow() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "ok",
            "adapter_ready": true,
            "login_status": true,
            "event_queue_size": 0,
            "audit_enabled": true,
            "version": "1.0",
            "environment": "test",
            "panic": false,
            "circuit_breaker_open": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/account/balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": {
                "account": "S1",
                "total_balance": 1000.0,
                "available_balance": 800.0
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/positions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": [{"account": "S1", "stk_code": "2330", "quantity": 10}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/orders/stock"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": {"client_order_id": "C1", "status": "SUBMITTED"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orders/C1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": {
                "client_order_id": "C1",
                "stk_code": "2330",
                "status": "SUBMITTED"
            }
        })))
        .mount(&server)
        .await;
    // Both new order and cancel hit the same TW endpoint; the matcher above is
    // enough for both calls in this integration test.

    let client: Box<dyn BrokerClient> = Box::new(tw_client(&server));
    run_unified_flow(client.as_ref()).await;
}
