//! Unified `BrokerClient` flow example.
//!
//! This example uses `Box<dyn BrokerClient>` to run the same
//! "submit order → get order → cancel → subscribe events" flow against both
//! the A-share and TW clients.
//!
//! Run with:
//! ```bash
//! cargo run --all-features --example unified_flow
//! ```

use broker_client::{
    AClient, BrokerClient, CancelOrderRequest, ClientConfig, OrderRequest, TwClient,
};

#[cfg(feature = "ws")]
use futures_util::StreamExt;
#[cfg(feature = "ws")]
use std::time::Duration;

async fn run_flow(client: &dyn BrokerClient, order: OrderRequest, cancel: CancelOrderRequest) {
    match client.health().await {
        Ok(health) => println!("health: {:?}", health.status),
        Err(err) => println!("health unavailable: {err}"),
    }

    match client.account().await {
        Ok(account) => println!("account: {:?}", account.account),
        Err(err) => println!("account unavailable: {err}"),
    }

    match client.positions().await {
        Ok(positions) => println!("positions: {} items", positions.len()),
        Err(err) => println!("positions unavailable: {err}"),
    }

    match client.submit_order(&order).await {
        Ok(status) => println!("submitted: {:?}", status.status),
        Err(err) => println!("submit unavailable: {err}"),
    }

    match client.get_order(&order.client_order_id).await {
        Ok(status) => println!("fetched: {:?}", status.status),
        Err(err) => println!("get_order unavailable: {err}"),
    }

    match client.cancel_order(&cancel).await {
        Ok(status) => println!("cancelled: {:?}", status.status),
        Err(err) => println!("cancel unavailable: {err}"),
    }

    #[cfg(feature = "ws")]
    match client.event_stream().await {
        Ok(mut events) => {
            println!("event stream connected; waiting for first event...");
            match tokio::time::timeout(Duration::from_secs(3), events.next()).await {
                Ok(Some(event)) => println!("first event: {event:?}"),
                Ok(None) => println!("event stream closed"),
                Err(_) => println!("no event within 3s"),
            }
        }
        Err(err) => println!("event stream unavailable: {err}"),
    }
}

fn tw_order() -> OrderRequest {
    OrderRequest::new(
        format!("unified-tw-{}", std::process::id()),
        "S98875005091",
        "2330",
        "B",
        500.0,
        1,
        "ROD",
        "LIMIT",
    )
}

fn tw_cancel() -> CancelOrderRequest {
    CancelOrderRequest::tw(
        format!("unified-tw-{}", std::process::id()),
        "S98875005091",
        "H00000",
        "2026/01/01",
        "2330",
        "B",
    )
}

fn a_order() -> OrderRequest {
    OrderRequest::a_new(
        format!("unified-a-{}", std::process::id()),
        "512100",
        "buy",
        3.305,
        100,
        true,
    )
}

fn a_cancel() -> CancelOrderRequest {
    CancelOrderRequest::new(format!("unified-a-{}", std::process::id()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- TW via unified trait ---");
    let tw: Box<dyn BrokerClient> = Box::new(TwClient::new(ClientConfig::tw_default()));
    run_flow(tw.as_ref(), tw_order(), tw_cancel()).await;

    println!("--- A-share via unified trait ---");
    let a: Box<dyn BrokerClient> = Box::new(AClient::new(ClientConfig::a_default()));
    run_flow(a.as_ref(), a_order(), a_cancel()).await;

    Ok(())
}
