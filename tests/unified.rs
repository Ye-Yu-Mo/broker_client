//! Integration tests for the unified `BrokerClient` trait.

use broker_client::{
    AClient, BrokerClient, CancelOrderRequest, ClientConfig, MockAccountInitRequest,
    MockPositionInit, OrderAction, OrderRequest, TwClient,
};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
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
        ..Default::default()
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

fn unified_mock_init() -> MockAccountInitRequest {
    MockAccountInitRequest {
        account: Some("MOCK-TEST".to_owned()),
        cash: 100_000.0,
        positions: vec![MockPositionInit {
            code: "2330".to_owned(),
            quantity: 10.0,
            available_quantity: Some(10.0),
            average_cost: Some(500.0),
        }],
        reset: Some(false),
    }
}

async fn run_unified_mock_flow(client: &dyn BrokerClient, order: OrderRequest) {
    let initialized = client
        .init_mock_account(&unified_mock_init())
        .await
        .unwrap();
    assert_eq!(initialized.cash, 100_000.0);
    assert_eq!(initialized.positions[0].code, "2330");
    assert_eq!(initialized.positions[0].quantity, 10.0);

    let queried = client.mock_account().await.unwrap();
    assert_eq!(queried.cash, 100_000.0);
    assert_eq!(queried.positions[0].average_cost, Some(500.0));

    let submitted = client.submit_order(&order).await.unwrap();
    assert_eq!(submitted.client_order_id.as_deref(), Some("M1"));
    assert_eq!(submitted.mock, Some(true));

    let fetched = client.get_order("M1").await.unwrap();
    assert_eq!(fetched.client_order_id.as_deref(), Some("M1"));
    assert_eq!(fetched.mock, Some(true));
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

#[tokio::test]
async fn a_client_runs_the_unified_mock_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/mock/init_account"))
        .and(body_json(json!({
            "cash": 100000.0,
            "positions": [{
                "symbol": "2330",
                "quantity": 10.0,
                "available_quantity": 10.0,
                "average_cost": 500.0
            }],
            "reset": false
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "cash": 100000.0,
            "positions": [{
                "symbol": "2330",
                "quantity": 10.0,
                "available_quantity": 10.0,
                "average_cost": 500.0
            }],
            "created_at_ms": 1,
            "updated_at_ms": 1
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/mock/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "cash": 100000.0,
            "positions": [{
                "symbol": "2330",
                "quantity": 10.0,
                "available_quantity": 10.0,
                "average_cost": 500.0
            }],
            "created_at_ms": 1,
            "updated_at_ms": 2
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/orders"))
        .and(body_json(json!({
            "client_order_id": "M1",
            "symbol": "2330",
            "side": "buy",
            "price": 500.0,
            "quantity": 1,
            "dry_run": false,
            "mock": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "client_order_id": "M1",
            "status": "Filled",
            "symbol": "2330",
            "mock": true,
            "filled_quantity": 1.0,
            "fill_price": 501.0,
            "filled_at_ms": 3
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/orders/M1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "client_order_id": "M1",
            "status": "Filled",
            "symbol": "2330",
            "mock": true
        })))
        .mount(&server)
        .await;

    let client: Box<dyn BrokerClient> = Box::new(a_client(&server));
    let order = OrderRequest::a_new("M1", "2330", "buy", 500.0, 1, false).with_mock(true);
    run_unified_mock_flow(client.as_ref(), order).await;
}

#[tokio::test]
async fn tw_client_runs_the_unified_mock_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mock/accounts/init"))
        .and(body_json(json!({
            "account": "MOCK-TEST",
            "cash": 100000.0,
            "positions": [{
                "stk_code": "2330",
                "quantity": 10,
                "avg_price": 500.0
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": {
                "account": "MOCK-TEST",
                "cash": 100000.0,
                "positions": {"2330": {"quantity": 10, "avg_price": 500.0}},
                "active": true,
                "created_at": "2026-09-14T00:00:00+00:00",
                "updated_at": "2026-09-14T00:00:00+00:00"
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/mock/accounts/MOCK%2DTEST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": {
                "account": "MOCK-TEST",
                "cash": 100000.0,
                "positions": {"2330": {"quantity": 10, "avg_price": 500.0}},
                "active": true,
                "created_at": "2026-09-14T00:00:00+00:00",
                "updated_at": "2026-09-14T00:00:01+00:00"
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/orders/stock"))
        .and(body_json(json!({
            "client_order_id": "M1",
            "action": "new",
            "account": "MOCK-TEST",
            "stk_code": "2330",
            "side": "B",
            "price": 500.0,
            "quantity": 1,
            "time_in_force": "ROD",
            "price_flag": "LIMIT",
            "mock": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": {"client_order_id": "M1", "status": "FILLED", "mock": true}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orders/M1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "ok",
            "data": {
                "client_order_id": "M1",
                "account": "MOCK-TEST",
                "stk_code": "2330",
                "status": "FILLED",
                "mock": true
            }
        })))
        .mount(&server)
        .await;

    let client: Box<dyn BrokerClient> = Box::new(tw_client(&server));
    let order =
        OrderRequest::new("M1", "MOCK-TEST", "2330", "B", 500.0, 1, "ROD", "LIMIT").with_mock(true);
    run_unified_mock_flow(client.as_ref(), order).await;
}
