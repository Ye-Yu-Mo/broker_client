# broker_client

Async Rust client library for the TW and A-share stock broker servers.

## Features

| Feature | Default | Description |
|---|---|---|
| `client-a` | yes | A-share server client (`AClient`) |
| `client-tw` | yes | TW server client (`TwClient`) |
| `ws` | no | WebSocket event stream and auto-reconnect support |

```bash
cargo build --no-default-features          # common core only
cargo build --features client-tw           # only TW client
cargo build --features client-a            # only A-share client
cargo build --features client-a,client-tw  # both (default)
cargo build --all-features                 # both + WebSocket
```

## Quick start

```rust,no_run
use broker_client::{AClient, TwClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Defaults match the documented test servers.
    let tw = TwClient::default();   // http://127.0.0.1:8000
    let a = AClient::default();     // http://127.0.0.1:8787

    println!("TW health: {:?}", tw.health_info().await);
    println!("A  health: {:?}", a.health_info().await);

    // A-share reads may return a cache-degraded response. Check the flag.
    if let Ok(account) = a.account().await {
        println!(
            "A account from_cache={}, cached_at={:?}",
            account.from_cache, account.cached_at
        );
    }

    // Dry-run A-share order: fills the form but does not confirm.
    let request = broker_client::AOrderRequest::new(
        "demo-order-1",
        "512100",
        "buy",
        3.305,
        100,
        true,
    );
    if let Ok(order) = a.submit_order(&request).await {
        println!("A dry-run order: {:?}", order.status);
    }
    Ok(())
}
```

Run the complete example (prints errors instead of panicking when a test
server is unavailable):

```bash
cargo run --example quickstart
cargo run --all-features --example quickstart   # also tries WebSocket events
```

## Configuration

```rust,no_run
use std::time::Duration;
use broker_client::{AClient, AuthMethod, ClientConfig};

let config = ClientConfig::new("http://127.0.0.1:8787")
    .token("your-token")
    .auth_method(AuthMethod::Bearer) // or AuthMethod::XAuthToken
    .timeout(Duration::from_secs(5))
    .retry(1)
    .ws_base_url("wss://127.0.0.1:9000/") // optional, defaults to base_url
    .user_agent("my-trader/1.0")
    .default_header("X-Environment", "UAT");

let client = AClient::new(config);
```

`ws_max_reconnect_attempts = 0` means the event stream reconnects forever.
The default is `5` finite reconnect attempts.

## WebSocket events

Enable the `ws` feature. A-share events keep the raw `data` and `timestamp_ms`
from the server; unknown event types are delivered as `AEvent::Unknown`.

```rust,no_run
# #[cfg(feature = "ws")]
# {
use broker_client::AClient;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = AClient::default();
    let mut events = a.event_stream().await?;
    if let Some(event) = events.next().await {
        println!("A event: {event:?}");
    }
    Ok(())
}
# }
```

The A server may mark read-only responses with `from_cache: true` and
`cached_at`. The client preserves these markers on `Cached<T>` values returned
by `account()`, `positions()`, `orders_by_status()`, `pnl()`,
`transactions()`, and the order list/detail methods.

## Error handling

All methods return `broker_client::Result<T>`. The `Error` enum preserves
transport errors, HTTP status/body, server API codes/messages/details and JSON
decode failures.

```rust,ignore
use broker_client::Error;

match client.health().await {
    Ok(_) => {}
    Err(Error::Timeout) => eprintln!("timed out"),
    Err(Error::Api { code, message, .. }) => eprintln!("{code}: {message}"),
    Err(err) => eprintln!("{err}"),
}
```
