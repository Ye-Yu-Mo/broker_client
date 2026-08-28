//! Minimal quickstart that exercises the client skeletons.
//!
//! Run with:
//! ```bash
//! cargo run --example quickstart
//! ```
//!
//! With default features this builds both `TwClient` and `AClient`. When only
//! one client feature is enabled, the example prints just that client's base
//! URL and health check.

#[cfg(feature = "client-a")]
use broker_client::AClient;
use broker_client::ClientConfig;
#[cfg(feature = "client-tw")]
use broker_client::TwClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "client-tw")]
    {
        // Default address matches the documented TW test server.
        let tw = TwClient::default();
        println!("TW base URL: {}", tw.config().base_url);
        match tw.health().await {
            Ok(value) => println!("TW health: {value}"),
            Err(err) => println!("TW health unavailable: {err}"),
        }
    }

    #[cfg(feature = "client-a")]
    {
        // Default address matches the documented A-share test server.
        let a = AClient::default();
        println!("A  base URL: {}", a.config().base_url);
        match a.health().await {
            Ok(value) => println!("A health: {value}"),
            Err(err) => println!("A health unavailable: {err}"),
        }
    }

    // Builder example showing how to point at a different base URL/token.
    let configured = ClientConfig::new("http://127.0.0.1:8787")
        .token("your-token")
        .retry(0);
    println!("configured retry={}", configured.retry);

    Ok(())
}
