# Changelog

变更记录

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)

版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)

---

## 版本号说明

- **主版本号（Major）**：不兼容的 API 变更或架构重构
- **次版本号（Minor）**：向后兼容的功能新增（新模块、新页面、新接口）
- **修订号（Patch）**：向后兼容的问题修正、小优化、文档更新

---

## [0.4.1] - 2026-09-17

### Added

- 测试：A 股 mock 账户未初始化时下单映射为 `Error::Api`，覆盖 `/v1` 错误中间件重写前后的两种错误体
- 测试：台股 `ApCode::AfterHours` 下单的请求体断言（补全 `ap_code` 枚举覆盖，此前只有 `ODD_LOT`）

### Changed

- 无

### Fixed

- `docs/a-client-api.md` 补齐 v0.4.0 新增的 `mock.account_changed` / `ws.lagged` WebSocket 事件，并消除「初始化会推送 `mock.account_changed`」与事件清单不一致的问题

### Security

- 无

---

## [0.4.0] - 2026-09-15

### Added

- A 股 `OrderRequest::mock(...)` 构造器与 `mock` 字段，对应服务端 `POST /v1/orders` 的 `mock=true` 模拟撮合（不提交真实委托，按 GUI 盘口模拟全量成交）
- A 股 `Order` 新增 `mock` / `fill_price` / `filled_at_ms` 字段，用于读取 mock 成交回执（`contract_id` 无类型化字段，仍保留在 `extra` 中不丢数据）
- `AClient::mock_account()` → `GET /v1/mock/account`
- `AClient::init_mock_account()` → `POST /v1/mock/init_account`
- 新增类型 `MockAccount` / `MockPosition` / `InitMockAccountRequest`，与 A server `state/mock.rs` 字段一一对应，并在 crate 根重导出
- 响应解析：A 股 `{"error": "..."}` 形式的错误体也映射为 `Error::Api`，与 `/v1` 错误中间件重写后的 `{code,message,detail}` 信封保持一致
- 统一 `OrderRequest` 新增 `mock: Option<bool>` 字段与 `with_mock(bool)` builder，两个 server 的 mock 下单均可从统一请求发起
- `HttpClient::delete_json()`：DELETE + JSON 解码，写操作不重试
- `TwClient::init_mock_account()` → `POST /api/v1/mock/accounts/init`
- `TwClient::mock_account()` → `GET /api/v1/mock/accounts/{account}`
- `TwClient::deactivate_mock_account()` → `DELETE /api/v1/mock/accounts/{account}`
- 新增类型 `MockAccountInitRequest` / `MockPositionInit` / `MockAccount`，请求侧与响应侧的持仓结构差异由两个类型分别表达
- 新增 TW `ApCode` 语义枚举（`Regular` / `OddLot` / `IntradayOddLot` / `AfterHours`）和 `OrderRequest::with_ap_code()`；请求序列化为语义字符串，反序列化同时兼容旧数字 `0/2/4/7`，非法值不回退
- `BrokerClient` 新增统一 `mock_account()` / `init_mock_account()`；A/TW 可通过同一份 trait-object 流程完成模拟账户初始化、查询、mock 下单和查单，默认实现让第三方 `BrokerClient` 实现保持源码兼容
- 新增统一 `MockAccountInitRequest` / `MockPositionInit` / `MockAccount`，由 A/TW 实现映射各自不同的字段名、持仓形状和时间格式；统一 `OrderStatus` 新增类型化 `mock` 字段
- 新增统一 `BrokerEvent::MockAccountChanged` 与 `BrokerEvent::WsLagged`，A 股 `AEvent` 可识别 `mock.account_changed` / `ws.lagged` 并保留 data/timestamp；`ws.lagged.data.skipped` 可直接读取
- TW 客户端继续将 A 股专属事件保留为 `Unknown`，不伪造 TW server 不提供的语义

### Changed

- A 股订单请求体新增 `mock` 字段；`mock=false` 时不序列化，不启用 mock 的请求体与 0.3.0 逐字节一致
- `AClient::submit_order` 增加 `mock && dry_run` 本地前置校验，直接返回 `Error::InvalidRequest` 而不发请求（服务端本就会 `400`，只是提前到客户端）
- A 股 `Order` → 统一 `OrderStatus` 的转换把 mock 成交回执折回 `extra`，统一视图不丢字段
- **破坏性变更（`types::OrderRequest` 结构体加字段）**：`types::OrderRequest` 新增 `mock` / `ap_code` 字段并改为 `derive(Default)`，`types::OrderAction` 同步 `derive(Default)`（缺省 `New`）。下游若用结构体字面量构造 `OrderRequest`，需补 `..Default::default()`（本仓库 `tests/unified.rs` 已改）。`AClient::submit_order` / `TwClient::submit_order` 现在把统一请求的 `mock` 透传给各自 server；`OrderRequest::a_new(...).with_mock(true)` 在 A 端不再被静默忽略。`ap_code=None` 时字段不序列化，服务端沿用 `REGULAR` 默认值
- **Breaking API change**：`a::Order` 和统一 `OrderStatus` 新增公共字段，旧的无 `..` struct literal 无法继续编译；`derive(Default)` 仅提供迁移写法，不能恢复旧 literal 兼容性。迁移请使用构造器或 `..Default::default()`
- **Breaking API change**：A 股专用 `a::OrderRequest` 新增公开 `mock: bool` 字段；旧版下游使用不带该字段的完整 struct literal 会触发 `E0063`。请改用 `AOrderRequest::new(...)` / `AOrderRequest::mock(...)` 构造器，或在字面量中补 `mock` 字段
- **Breaking API change**：`AEvent` / `BrokerEvent` 新增事件变体，穷举 `match` 必须补 `MockAccountChanged` / `WsLagged`，或增加 wildcard 分支
- `OrderStatus` 新增的可选字段在值为 `None` 时不再序列化为 `null`，保持旧状态 JSON 输出形状；字段有值时仍正常输出

- **Breaking API change**：`Error::Api` 新增 `status: Option<u16>`，用于保留 HTTP API 错误状态并恢复 GET 429/5xx 重试；下游显式解构 `Error::Api` 时需增加该字段或使用 `..`
- TW `submit_stock_order`、`order.updated` / 查单响应现在会从嵌套 `data` 提升 `mock`、`fill_price`、`filled_qty`（兼容 `filled_quantity`）到统一 `OrderStatus`，同时保留原始 payload
- 修复 TW mock 统一初始化在 `i64::MAX as f64` 舍入边界上的超范围数量转换
- TW mock 账户名校验与服务端对齐 `min_length=6` / `max_length=64`

### Security

- 无

---

## [0.3.0] - 2026-08-29

### Added

- 新增统一类型模型 `src/types.rs`：`OrderRequest`、`OrderStatus`、`Position`、`Account`、`BrokerEvent`、`CancelOrderRequest`
- 新增 `BrokerClient` trait，`AClient` / `TwClient` 均可作为 `Box<dyn BrokerClient>` 使用
- 新增 `examples/unified_flow.rs` 统一 trait 示例
- 新增 A/TW WebSocket `BrokerEvent` 统一事件流转换测试

### Changed

- TW `OrderRequest`、`OrderStatus`、`Position` 改为统一类型的 alias，保留旧构造器与字段

### Fixed

- 修复 rustdoc 链接警告

### Security

- 无

---

## [0.2.1] - 2026-08-29

### Added

- 无

### Changed

- 更新 TW WebSocket 文档：认证仅使用 `Authorization: Bearer` header，不再通过 URL query 传递 token

### Fixed

- WS 重连计数改为成功收到首个事件后才归零，避免“连接成功但未收到任何事件”时错误重置重连上限
- 移除 `Mutex::lock().expect(...)` panic 路径，Mutex 中毒时安全恢复继续操作

### Security

- WebSocket token 不再出现在 URL query 中，仅通过 `Authorization: Bearer` header 传递

---

## [0.2.0] - 2026-08-28

### Added

- A 股 `Position` 新增 `today_qty`（今仓）和 `yesterday_qty`（昨仓）字段
- `AClient` 新增 `notify_test()`，对应 `POST /v1/notify/test`
- 新增 `NotifyTestResponse` 类型
- 更新 `docs/a-client-api.md`，同步 A 股 server v0.3.0 接口变更

### Changed

- 无（向后兼容新增）

### Fixed

- 无

### Security

- 无

---

## [0.1.0] - 2026-08-28

### Added

#### M1: 公共核心与可测试骨架

- 新增统一异步 HTTP 客户端：
  - 连接池复用
  - 可配置 timeout
  - 仅对幂等 GET 自动重试，POST/PUT/DELETE 等写操作不自动重试
  - 自动注入 `Authorization: Bearer` / `X-Auth-Token`
  - 自动生成或透传 `X-Request-ID`
  - 统一 `User-Agent` 与默认 headers
- 新增 `ClientConfig`：
  - `base_url`、`token`、`auth_method`、`timeout`、`retry`
  - `user_agent`、`default_headers`
  - WebSocket 重连相关配置
- 新增统一错误类型 `Error` / `Result<T>`：
  - transport / timeout / HTTP status / API error / decode / WebSocket / invalid URL / invalid request
  - 保留原始 HTTP body 与服务端 `code/message/detail`
- 新增响应解析：
  - TW 成功 envelope `{ code, message, data }`
  - TW 错误 envelope `{ detail: { code, message, detail } }`
  - A 股错误 body `{ code, message, detail }`
- 新增 feature 开关：
  - `client-a`
  - `client-tw`
  - `ws`
- 新增 `TwClient::default()` / `AClient::default()`：
  - TW 默认 `http://127.0.0.1:8000`
  - A 股默认 `http://127.0.0.1:8787`
- 新增基于 `wiremock` 的 mock server 测试
- 新增 README 与快速开始示例

#### M2: TW server client 完整实现

- 新增 `TwClient` 完整 HTTP 接口：
  - 登录 / 登出 / 会话状态
  - 账户 / 持仓 / 结算 / 盈亏 / 报表
  - 行情订阅 / 退订 / 已订阅列表 / 快照 / 分时 / 分价 / K 线 / 个股资讯
  - 下单 / 撤单 / 改价 / 改量 / 订单查询
  - panic / resume / recovery
- 新增 TW 类型模型：
  - `SessionInfo`、`Position`、`Balance`、`Settlement`
  - `Pnl*`、`RealReport*`、`OrderTradeReport`
  - `QuoteSnapshot`、`Tick`、`Kline`、`StockInfo`
  - `OrderRequest`、`OrderStatus`、`RecoveryItem`
- 新增 `TwEvent`：
  - `welcome`、`Login`、`RR_RealReport`、`RR_RealReportMerge`
  - `real_report`、`real_report_merge`、`order.updated`、`quote.updated`
  - `heartbeat`、订阅原始事件、未知事件透传
- 新增 WebSocket：
  - `connect_ws()`
  - `event_stream()` 自动重连
  - 重连后通过 HTTP 重新订阅已订阅列表
- 新增 `ClientConfig::ws_base_url`，支持 HTTP 与 WebSocket 分离配置
- 新增订单幂等语义保护：client 层不隐式重发

#### M3: A server client 完整实现与整体收尾

- 新增 `AClient` 完整 HTTP 接口：
  - 账户 / 持仓 / 盈亏 / 资金流水
  - 委托列表 / 成交列表 / 订单状态筛选 / 订单详情
  - health / metrics
  - refresh / 下单 / 撤单 / 改单
  - panic / resume
- 新增 A 股类型模型：
  - `AccountFunds`、`Position`、`Order`、`Trade`、`Pnl`
  - `Transaction`、`Health`、`OrderRequest`
  - `CancelRequest`、`ReplaceRequest`、`RefreshResponse`
- 新增 `Cached<T>`：
  - 保留 `from_cache` / `cached_at` 缓存降级标记
  - 同时支持直接响应与 `{ data, from_cache, cached_at }` 包装响应
- 新增 `AEvent`：
  - `order.updated`、`position.changed`、`account.changed`
  - `account.balance_changed`、`query.cache_hit`、`replace.updated`
  - `order.no_mapping`、`order.manual_review`、`risk.panic`
  - `health.changed`、未知事件透传
  - 所有事件保留 `timestamp_ms` 与原始 `data`
- 新增 A 股 WebSocket：
  - `connect_ws()`
  - `event_stream()` 自动重连
- 新增统一示例 `examples/quickstart.rs`，同时覆盖 A 股与 TW server
- 新增 README 完整使用文档
- 新增 `.github/workflows/ci.yml` CI：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`

### Changed

- 无（首次发布）

### Fixed

- 无（首次发布）

### Security

- token 通过 `Authorization` 请求头传递
- WebSocket URL 中的 token 使用百分号编码
- 写操作默认不自动重试，避免重复下单/撤单/改单
