//! Unified quickstart for both the A-share and TW server clients.
//!
//! Run with:
//! ```bash
//! cargo run --example quickstart
//! cargo run --all-features --example quickstart   # also listens to WebSocket
//! ```
//!
//! The example never panics if a test server is unavailable; it prints the
//! error and continues.

use broker_client::ClientConfig;
#[cfg(feature = "client-a")]
use broker_client::{AClient, AOrderRequest};
#[cfg(feature = "client-tw")]
use broker_client::{OrderRequest, TwClient};
#[cfg(feature = "ws")]
use futures_util::StreamExt;
#[cfg(feature = "ws")]
use std::time::Duration;

#[cfg(feature = "client-a")]
async fn demo_a() {
    let a = AClient::default();
    println!("A  base URL: {}", a.config().base_url);

    match a.health().await {
        Ok(value) => println!("A  health: {value}"),
        Err(err) => println!("A  health unavailable: {err}"),
    }

    // Read account (may be a cached downgrade response).
    match a.account().await {
        Ok(account) => println!(
            "A  account: total_asset={:?}, from_cache={}",
            account.data.total_asset, account.from_cache
        ),
        Err(err) => println!("A  account unavailable: {err}"),
    }

    // Submit a dry-run order: this only fills the form, never confirms.
    let request = AOrderRequest::new(
        format!("demo-a-{}", std::process::id()),
        "512100",
        "buy",
        3.305,
        100,
        true,
    );
    match a.submit_order(&request).await {
        Ok(order) => println!("A  dry-run order status: {:?}", order.status),
        Err(err) => println!("A  dry-run order unavailable: {err}"),
    }

    #[cfg(feature = "ws")]
    match a.event_stream().await {
        Ok(mut events) => {
            println!("A  event stream connected; waiting for first event...");
            match tokio::time::timeout(Duration::from_secs(3), events.next()).await {
                Ok(Some(event)) => println!("A  first event: {event:?}"),
                Ok(None) => println!("A  event stream closed"),
                Err(_) => println!("A  no event within 3s"),
            }
        }
        Err(err) => println!("A  event stream unavailable: {err}"),
    }
}

#[cfg(feature = "client-tw")]
async fn demo_tw() {
    let tw = TwClient::default();
    println!("TW base URL: {}", tw.config().base_url);

    match tw.health().await {
        Ok(value) => println!("TW health: {value}"),
        Err(err) => println!("TW health unavailable: {err}"),
    }

    match tw.session_status().await {
        Ok(status) => println!("TW session: logged_in={:?}", status.logged_in),
        Err(err) => println!("TW session unavailable: {err}"),
    }

    // The example intentionally uses a unique client_order_id and relies on
    // the caller to run it against a test server where the account exists.
    let request = OrderRequest::new(
        format!("demo-tw-{}", std::process::id()),
        "S98875005091",
        "2330",
        "B",
        500.0,
        1,
        "ROD",
        "LIMIT",
    );
    match tw.submit_stock_order(&request).await {
        Ok(status) => println!("TW order status: {:?}", status.status),
        Err(err) => println!("TW order unavailable: {err}"),
    }

    #[cfg(feature = "ws")]
    match tw.event_stream().await {
        Ok(mut events) => {
            println!("TW event stream connected; waiting for first event...");
            match tokio::time::timeout(Duration::from_secs(3), events.next()).await {
                Ok(Some(event)) => println!("TW first event: {event:?}"),
                Ok(None) => println!("TW event stream closed"),
                Err(_) => println!("TW no event within 3s"),
            }
        }
        Err(err) => println!("TW event stream unavailable: {err}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "client-a")]
    demo_a().await;

    #[cfg(feature = "client-tw")]
    demo_tw().await;

    // Builder example showing how to point at a different base URL/token.
    let configured = ClientConfig::new("http://127.0.0.1:8787")
        .token("your-token")
        .retry(0);
    println!("configured retry={}", configured.retry);

    Ok(())
}
