# broker_client

[![CI](https://github.com/Ye-Yu-Mo/broker_client/actions/workflows/ci.yml/badge.svg)](https://github.com/Ye-Yu-Mo/broker_client/actions/workflows/ci.yml)

面向两个券商交易 server 的 Rust 异步客户端库：

- **A 股 server**（`AClient`）— [stock_broker_a_server](https://github.com/Ye-Yu-Mo/stock_broker_a_server)：同花顺 macOS 自动化交易服务
- **台股 server**（`TwClient`）— [stock-broker-tw-server](https://github.com/Ye-Yu-Mo/stock-broker-tw-server)：台股交易服务

库提供统一的异步 HTTP 基础、类型化模型、WebSocket 事件流、自动重连、缓存降级元数据，以及安全的写操作默认行为。

## 功能特性

| Feature | 默认 | 说明 |
|---|---|---|
| `client-a` | 是 | A 股 server 客户端（`AClient`） |
| `client-tw` | 是 | 台股 server 客户端（`TwClient`） |
| `ws` | 否 | WebSocket 事件流与自动重连支持 |

```bash
cargo build --no-default-features          # 只编译公共核心
cargo build --features client-tw           # 只编译台股客户端
cargo build --features client-a            # 只编译 A 股客户端
cargo build --features client-a,client-tw  # 同时编译两个客户端（默认）
cargo build --all-features                 # 两个客户端 + WebSocket
```

## 支持的 server

| Client | Server | 默认地址 |
|---|---|---|
| `AClient` | [A 股 / 同花顺 server](https://github.com/Ye-Yu-Mo/stock_broker_a_server) | `http://127.0.0.1:8787` |
| `TwClient` | [stock-broker-tw-server](https://github.com/Ye-Yu-Mo/stock-broker-tw-server) | `http://127.0.0.1:8000` |

两个客户端都支持 `Authorization: Bearer <token>` 和 `X-Auth-Token: <token>`。
默认使用 `Bearer`；A 股 server 同时接受 `X-Auth-Token`。

A 股客户端额外支持：

- 持仓 `today_qty` / `yesterday_qty`（今仓 / 昨仓）
- `notify_test()`：发送飞书测试报警，对应 `POST /v1/notify/test`

## 快速开始

```rust,no_run
use broker_client::{AClient, TwClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 默认地址对应文档中的测试 server。
    let tw = TwClient::default();   // http://127.0.0.1:8000
    let a = AClient::default();     // http://127.0.0.1:8787

    println!("TW health: {:?}", tw.health_info().await);
    println!("A  health: {:?}", a.health_info().await);

    // A 股只读接口可能返回缓存降级响应，可以通过 from_cache 判断。
    if let Ok(account) = a.account().await {
        println!(
            "A account from_cache={}, cached_at={:?}",
            account.from_cache, account.cached_at
        );
    }

    // A 股 dry-run 下单：只填写表单并回读校验，不会点击确认。
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

运行完整示例（server 不可用时只打印错误，不会 panic）：

```bash
cargo run --example quickstart
cargo run --all-features --example quickstart   # 同时尝试 WebSocket 事件
```

## 配置

```rust,no_run
use std::time::Duration;
use broker_client::{AClient, AuthMethod, ClientConfig};

let config = ClientConfig::new("http://127.0.0.1:8787")
    .token("your-token")
    .auth_method(AuthMethod::Bearer) // 或 AuthMethod::XAuthToken
    .timeout(Duration::from_secs(5))
    .retry(1)
    .ws_base_url("wss://127.0.0.1:9000/") // 可选，默认从 base_url 推导
    .user_agent("my-trader/1.0")
    .default_header("X-Environment", "UAT");

let client = AClient::new(config);
```

`ws_max_reconnect_attempts = 0` 表示事件流无限重连。
默认值为 `5` 次有限重连。

## WebSocket 事件

启用 `ws` feature 后，两个客户端都提供：

- `connect_ws()` — 单条 WebSocket 连接
- `event_stream()` — 自动重连的事件流

A 股事件会保留 server 返回的原始 `data` 和 `timestamp_ms`；未知事件会以
`AEvent::Unknown` 透传。台股事件会解析为 `TwEvent`，未知类型同样保留。

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

## 缓存降级元数据

A 股 server 可能在只读接口响应中标记 `from_cache: true` 和 `cached_at`。
客户端会通过 `Cached<T>` 保留这些标记，涉及的接口包括：

- `account()`
- `positions()`
- `pnl()`
- `transactions()`
- `list_orders()` / `list_trades()` / `orders_by_status()`
- `get_order()`

## 写操作安全

写操作（`submit_order`、`cancel_order`、`replace_order`、
`submit_stock_order`、`panic`、`resume` 等）**默认不会自动重试**，
避免重复下单、撤单或改单。

A 股 `replace_order` 不会隐式实现“先撤单再重新下单”；如果需要这种流程，
由调用方显式实现。

## 错误处理

所有方法返回 `broker_client::Result<T>`。`Error` 枚举会保留传输错误、
HTTP status/body、服务端 API code/message/detail 和 JSON 解析错误。

```rust,ignore
use broker_client::Error;

match client.health().await {
    Ok(_) => {}
    Err(Error::Timeout) => eprintln!("请求超时"),
    Err(Error::Api { code, message, .. }) => eprintln!("{code}: {message}"),
    Err(Error::InvalidRequest(message)) => eprintln!("请求参数错误: {message}"),
    Err(err) => eprintln!("{err}"),
}
```

## 项目结构

```text
src/
├── auth.rs            # Bearer / X-Auth-Token 辅助
├── config.rs          # 共享 ClientConfig 和 URL 工具
├── error.rs           # 统一 Error / Result
├── http.rs            # 共享异步 HTTP 客户端
├── response.rs        # TW / A 股响应 envelope 解析
└── client/
    ├── a/             # A 股 server 客户端
    │   ├── mod.rs
    │   ├── types.rs
    │   └── ws.rs
    └── tw/            # 台股 server 客户端
        ├── mod.rs
        ├── types.rs
        └── ws.rs
```

## 开发

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo doc --no-deps --all-features
```

## 文档

- `docs/a-client-api.md` — A 股 server API 文档
- `docs/tw-client-api.md` — 台股 server API 文档
- `CHANGELOG.md` — 变更记录

## License

MIT
