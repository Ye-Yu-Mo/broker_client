# broker_client

Async Rust client library for the TW and A-share stock broker servers.

## Features

| Feature | Default | Description |
|---|---|---|
| `client-a` | yes | A-share server client (`AClient`) |
| `client-tw` | yes | TW server client (`TwClient`) |
| `ws` | no | WebSocket dependencies (reserved for later milestones) |

```bash
cargo build --no-default-features          # common core only
cargo build --features client-tw           # only TW client
cargo build --features client-a            # only A-share client
cargo build --features client-a,client-tw  # both (default)
```

## Quick start

```rust,ignore
use broker_client::{AClient, TwClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Defaults match the documented test servers.
    let tw = TwClient::default();   // http://127.0.0.1:8000
    let a = AClient::default();     // http://127.0.0.1:8787

    // Both health endpoints are unauthenticated in M1.
    println!("TW health: {:?}", tw.health().await);
    println!("A  health: {:?}", a.health().await);
    Ok(())
}
```

Run the complete example:

```bash
cargo run --example quickstart
```

## Configuration

```rust,ignore
use std::time::Duration;
use broker_client::{AClient, AuthMethod, ClientConfig};

let config = ClientConfig::new("http://127.0.0.1:8787")
    .token("your-token")
    .auth_method(AuthMethod::Bearer) // or AuthMethod::XAuthToken
    .timeout(Duration::from_secs(5))
    .retry(1)
    .user_agent("my-trader/1.0")
    .default_header("X-Environment", "UAT");

let client = AClient::new(config);
```

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
