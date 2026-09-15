# TODO-M1: broker_client 适配 A / TW server 最新一轮更新

---

## 目标

- **本任务要解决的问题**：`stock_broker_a_server` 已发到 v0.4.0、`stock_broker_tw_server` 已发到 v0.1.10，两个 server 在这一轮更新里各自新增了「请求级 mock 下单标记 + 独立模拟账户管理」能力，以及一批新字段/新事件。`broker_client` 停在 0.3.0，**完全没有 `mock` 字段透传、没有 mock 账户接口、没有 `ap_code` 字段**，导致：
  1. `market_webserver` / `order_router` 未来无法用统一客户端做不碰真钱的端到端下单测试；
  2. 台股 `ap_code`（盘后/零股/盘中零股）无法通过本库下单，只能绕开本库裸发 HTTP；
  3. A 股 mock 成交回执（`fill_price` / `filled_at_ms`）与新增 WS 事件在客户端被丢弃或降级为 `Unknown`。
- **所属 Milestone**：M1（本仓库 TODO 序列第 1 篇）。对应根 `PLAN.md` 的 **M2（broker_client 统一抽象）收尾** + **M5.5 第 2 项（broker server mock 撮合）的客户端侧前置**。

> 说明：本轮只做**透传与适配**，不改任何现有方法签名语义、不动写操作重试策略。

---

## 功能1: A 股 mock 撮合透传（mock 标记 + 模拟账户管理）

### 修改1: `client/a/types.rs::OrderRequest` 增加 `mock` 字段

- **改什么**：在 A 股专用 `OrderRequest` 结构体（`src/client/a/types.rs:364`）增加 `#[serde(default)] pub mock: bool`；`OrderRequest::new()` 增加一个构造入口或新增 `OrderRequest::mock(...)` 构造器，保持原 `new()` 签名不变。
- **为什么改**：A server v0.4.0 的 `POST /v1/orders` 请求体已支持 `mock` 字段（`stock_broker_a_server/src/broker/order.rs:70`），且服务端硬性约束 `mock` 与 `dry_run` 互斥（`engine/order.rs:408-412`，同时为 true 直接 400 `"mock 与 dry_run 不能同时启用"`）。客户端不带这个字段，就等于放弃了整条 mock 链路。
- **预期结果**：`ac.submit_order(&OrderRequest::mock("c1", "512100", "buy", 3.305, 100))` 能发出 `{"mock": true, "dry_run": false, ...}` 的请求体；`AClient::submit_order` 在本地就对 `mock && dry_run` 做校验并返回 `Error::InvalidRequest`，不发无用请求。

### 修改2: `client/a/types.rs::Order` 补齐 mock 成交回执字段

- **改什么**：`Order` 增加 `mock: Option<bool>`、`fill_price: Option<f64>`、`filled_at_ms: Option<u64>`（`filled_quantity` 已存在）。
- **为什么改**：A server 的 `OrderRecord`（`engine/state.rs:192-199`）在 v0.4.0 新增了 `mock`、`fill_price`、`filled_at_ms`。当前客户端靠 `extra: Value` 兜底不丢数据，但调用方拿不到类型化字段来判断"这笔到底是不是模拟成交、成交在什么价"。
- **预期结果**：mock 下单返回的 `Order` 能直接读到 `fill_price` / `filled_at_ms`；dry-run 订单三个字段均为 `None`。

### 修改3: `AClient` 新增模拟账户读写接口

- **改什么**：新增三个方法，对应服务端两条路由（`stock_broker_a_server/src/api/http.rs:337-338`）：
  - `mock_account(&self) -> Result<MockAccount>` → `GET /v1/mock/account`
  - `init_mock_account(&self, request: &InitMockAccountRequest) -> Result<MockAccount>` → `POST /v1/mock/init_account`
  - 配套类型 `MockAccount { cash, positions: Vec<MockPosition>, created_at_ms, updated_at_ms }`、`MockPosition { symbol, quantity, available_quantity, average_cost }`、`InitMockAccountRequest { cash, positions, reset }`（字段与服务端 `state/mock.rs:10-35` 一一对应）。
- **为什么改**：服务端已有这两个接口，客户端零覆盖；没有它就无法为 mock 下单准备初始资金/持仓，mock 链路实际不可用。
- **预期结果**：`init_mock_account` 未初始化时返回 200，已初始化且 `reset=false` 时返回 409 映射为 `Error::Api{code:...}`；`mock_account()` 在未初始化时把 404 映射为 `Error::Api`；返回结构的所有字段均可类型化访问。
- **注意**：这两个接口的错误响应是 `{"error": msg}`，经服务端 `/v1` 错误中间件（`api/http.rs:408-444`）重写为 `{code,message,detail}` 信封，客户端按现有 `parse_a_error_body` 路径即可正确解析 —— 但必须**验证**，不能假设。

### 影响文件

- `src/client/a/types.rs`
- `src/client/a/mod.rs`
- `docs/a-client-api.md`
- `CHANGELOG.md`

---

## 功能2: 台股 mock 撮合透传（mock 标记 + 模拟账户管理）

### 修改1: 统一 `types::OrderRequest` 增加 mock 语义

- **改什么**：在 `src/types.rs::OrderRequest` 增加 `#[serde(default, skip_serializing_if = "Option::is_none")] pub mock: Option<bool>`；`OrderRequest::new(...)`（TW 侧）与 `a_new(...)` 两个构造器各加参数或新增 `with_mock(self, bool)` builder；`Tw encode` 路径确保 `mock: true` 进入请求体。
- **为什么改**：TW server 0.1.2 起 `StockOrderPayload` 就有 `mock: bool = False`（`stock_broker_tw_server/src/stock_broker_tw/api/http.py:46`），且 mock 账户必须使用 `MOCK-` 命名空间（`state/store.py:40` 正则 `^MOCK-[A-Za-z0-9][A-Za-z0-9_.-]*$`）。客户端不发这个字段就永远走真实下单。
- **预期结果**：`submit_stock_order` 携带 `mock: true` 时服务端走 `_execute_mock`，返回本地模拟成交；`mock` 为 `None` 时请求体字节与当前完全一致（可加字节级回归断言）。
- **风险**：这是**破坏性结构体变更**（见"兼容性检查"），必须用 builder 而非改 `new()` 签名，把破坏面压到最小。

### 修改2: `TwClient` 新增模拟账户三接口

- **改什么**：新增三个方法，对应服务端三条路由（`api/http.py:619,639,654`）：
  - `init_mock_account(&self, request: &MockAccountInitRequest) -> Result<MockAccount>` → `POST /api/v1/mock/accounts/init`
  - `mock_account(&self, account: &str) -> Result<MockAccount>` → `GET /api/v1/mock/accounts/{account}`
  - `deactivate_mock_account(&self, account: &str) -> Result<MockAccount>` → `DELETE /api/v1/mock/accounts/{account}`
  - `account` 路径段必须走 `encode_path_segment`（与 `get_order` 一致）。
- **为什么改**：服务端有、客户端零覆盖，与功能1 同因。
- **预期结果**：三个方法路径、方法与信封均正确；`GET` 未初始化账户时 404 `MOCK_ACCOUNT_NOT_FOUND` 映射为 `Error::Api`；`account` 非法命名空间时 400 `INVALID_MOCK_ACCOUNT` 同样映射为 `Error::Api`。

### 修改3: 新增 TW 模拟账户类型

- **改什么**：新增 `MockAccountInitRequest { account, cash, positions: Vec<MockPositionInit> }`、`MockPositionInit { stk_code, quantity, avg_price }`、`MockAccount { account, cash, positions: Value, active, created_at, updated_at }`。
- **为什么改**：服务端 init 请求体字段是 `stk_code/quantity/avg_price`（`api/http.py:49-67`），响应体字段是 `account/cash/positions/active/created_at/updated_at`（`state/store.py:453-461`）——**请求与响应的持仓字段名不同**（`stk_code` vs map key），不能用同一个类型糊过去，必须两个类型。
- **预期结果**：`positions` 在响应侧是账户→持仓的 map，保持 `serde_json::Value` 原样透传（不做自定义反序列化，避免猜错结构）；init 请求侧强类型且 `account` 在客户端先做 `MOCK-` 前缀校验，不合法直接 `Error::InvalidRequest`。

### 影响文件

- `src/types.rs`
- `src/client/tw.rs`
- `src/client/tw/types.rs`
- `docs/tw-client-api.md`
- `CHANGELOG.md`

---

## 功能3: 台股 `ap_code` 语义化字段

### 修改1: 新增 `ApCode` 枚举

- **改什么**：新增 `#[derive(Serialize, Deserialize)] #[serde(rename_all = "SCREAMING_SNAKE_CASE")] pub enum ApCode { Regular, OddLot, IntradayOddLot, AfterHours }`，并实现 `From<u8>` / `TryFrom<i32>` 兼容旧数字 `0/2/4/7`；序列化时输出字符串（与服务端 `ap_code: ApCode | int | str` 的双向兼容一致）。
- **为什么改**：TW server 0.1.3（2026-09-09）新增了语义化 `ap_code`，服务端同时兼容旧数字（`CHANGELOG.md:110`，`engine/state.py:53-97`）。客户端 0.3.0 完全没有这个字段，导致盘后交易/零股下单无法通过本库发起。
- **预期结果**：`ApCode::AfterHours` 序列化为 `"AFTER_HOURS"`；传入数字 `7` 也能反序列化为 `ApCode::AfterHours`（旧调用方向后兼容）。

### 修改2: 接入统一 `OrderRequest` 与 TW 下单方法

- **改什么**：`types::OrderRequest` 增加 `#[serde(default, skip_serializing_if = "Option::is_none")] pub ap_code: Option<ApCode>`；`TwClient::submit_stock_order` 原样透传；`OrderRequest::new()` 默认 `ap_code: None`（服务端默认 `REGULAR`）。
- **为什么改**：服务端 `StockOrderPayload.ap_code` 默认 `REGULAR`（`api/http.py:39`），客户端省略该字段时行为不变，因此这是纯增量。
- **预期结果**：不传 `ap_code` 时请求体与当前字节一致；传 `Some(ApCode::OddLot)` 时请求体出现 `"ap_code": "ODD_LOT"`。

### 影响文件

- `src/types.rs`
- `src/client/tw.rs`
- `src/client/tw/types.rs`
- `docs/tw-client-api.md`

---

## 功能4: 统一 `BrokerClient` trait 补齐 mock 能力

### 修改1: trait 增加模拟账户方法

- **改什么**：`src/client/broker.rs::BrokerClient` 增加两个方法：
  - `async fn mock_account(&self) -> Result<MockAccount>;`
  - `async fn init_mock_account(&self, request: &MockAccountInitRequest) -> Result<MockAccount>;`
  两个 server 的入参形状不同（A 是 `{cash, positions:[{symbol,...}], reset}`，TW 是 `{account, cash, positions:[{stk_code,...}]}`），统一类型取超集：`MockAccountInitRequest { account: Option<String>, cash: f64, positions: Vec<MockPositionInit>, reset: Option<bool> }`，`MockPositionInit { code: String, quantity: f64, available_quantity: Option<f64>, average_cost: Option<f64> }`，由各实现映射到自己的 wire 字段。
- **为什么改**：`market_webserver` 的下单接口端到端测试是 PLAN.md M5.5 的验收项，它只会依赖 `Box<dyn BrokerClient>`。如果 mock 能力只做在 `AClient` / `TwClient` 具体类型上，统一抽象就等于漏了一块。
- **预期结果**：`Box<dyn BrokerClient>` 能用同一份代码对 A/TW 两个 server 初始化模拟账户并下单，与 `tests/unified.rs::run_unified_flow` 同构。

### 修改2: 统一类型增加 mock 字段（**并记录债务**）

- **改什么**：`types::OrderRequest` 与 `types::OrderStatus` 各增加 `mock: Option<bool>`；同时在模块文档注释里明确写下：**统一 `OrderRequest` 目前是 A/TW 两套 wire 字段的并集（`side` 一个字段装 `buy/sell` 与 `B/S` 两套语义、`account` 用空串代表"没有"、`symbol`/`dry_run` 靠 `skip_serializing` 假装不存在），第 N 次往上面加 `Option` 时应当重构为 `enum`/trait**。
- **为什么改**：这是本轮改动里唯一会**让现有数据结构变差**的一步。不加就丢能力，加了就继续往错误的数据结构上叠补丁。诚实的做法是加，同时把债务写进代码里，而不是留到下次再"发现"。
- **预期结果**：字段可用；`src/types.rs` 模块头注释里有一段明确的债条与重构方向（建议：`enum UnifiedOrder { A(AOrderRequest), Tw(TwStockOrderRequest) }`）。

### 影响文件

- `src/client/broker.rs`
- `src/types.rs`
- `src/client/a/mod.rs`
- `src/client/tw.rs`
- `tests/unified.rs`

---

## 功能5: 新增 WebSocket 事件类型映射

### 修改1: `AEvent` 补齐 v0.4.0 新事件

- **改什么**：`src/client/a/types.rs::AEvent` 增加 `MockAccountChanged { data, timestamp_ms }` 与 `WsLagged { data, timestamp_ms }` 两个变体（对应服务端 `mock.account_changed`、`ws.lagged`）。
- **为什么改**：A server v0.4.0 新增这两个事件（`engine/events.rs:11-56`）。当前客户端会把它们降级为 `AEvent::Unknown`——**不丢数据但也不可区分**，`ws.lagged`（广播通道容量 256，消费滞后时带 `skipped`）尤其重要：它意味着**客户端已经漏事件了**，降级成 `Unknown` 等于把这个信号静默掉。
- **预期结果**：两个事件可被 `match` 到具体变体；`WsLagged` 的 `data.skipped` 可读；未知事件仍走 `Unknown` 透传。

### 修改2: `BrokerEvent` 增加对应统一变体

- **改什么**：`types::BrokerEvent` 增加 `MockAccountChanged` 与 `WsLagged` 变体；A/TW 两侧的 `From` 映射补齐；TW 侧若无对应事件则不产生该变体（保持 `Unknown`）。
- **为什么改**：统一事件流目前没有"丢事件"的表达能力，订阅方无法感知数据缺口。
- **预期结果**：`Box<dyn BrokerClient>::event_stream()` 能产出 `BrokerEvent::WsLagged`；A/TW 两实现的事件转换测试均覆盖新变体。

### 影响文件

- `src/client/a/types.rs`
- `src/types.rs`
- `src/client/a/ws.rs`
- `src/client/tw/types.rs`

---

## 测试计划

- **单元测试**：
  - A `OrderRequest` 含 `mock: true` 时的 JSON 精确形状；`mock && dry_run` 在客户端被拒且不发请求。
  - TW `OrderRequest` 省略 `mock`/`ap_code` 时序列化结果与 0.3.0 **逐字节一致**（防止静默破坏 wire 兼容）。
  - `ApCode` 与 `0/2/4/7` 的双向转换，以及大写下划线字符串序列化。
  - `Order` 新字段（`mock`/`fill_price`/`filled_at_ms`）反序列化，缺失时为 `None`。
- **集成测试**（wiremock）：
  - `AClient::mock_account` → `GET /v1/mock/account`；404 `{"error":...}` 与 v1 信封两种错误体都能映射为 `Error::Api`。
  - `AClient::init_mock_account` → `POST /v1/mock/init_account`，覆盖 200 / 409 / 400 三种状态。
  - `TwClient` 三接口的路径、方法、信封解析，以及 404 `MOCK_ACCOUNT_NOT_FOUND` 映射。
  - `Box<dyn BrokerClient>` 对 A/TW 各跑一次「init mock 账户 → mock 下单 → 查单」全流程（扩展 `tests/unified.rs`）。
- **回归测试**：
  - 现有全部测试必须零改动通过（`cargo test --locked --all-features`）。
  - 新增"缺省字段请求体字节一致性"断言，锁死向后兼容。
  - `cargo clippy --locked --all-targets --all-features -- -D warnings` + `cargo fmt --check` + `cargo doc --no-deps --all-features` 全绿。
- **边界条件测试**：
  - TW mock 账户名不匹配 `^MOCK-[A-Za-z0-9][A-Za-z0-9_.-]*$` → 客户端本地拒绝（`Error::InvalidRequest`），不发请求。
  - A mock 账户未初始化时下单 → 服务端 `Rejected`，客户端返回 `Error::Api` 而非 panic。
  - TW `ap_code` 传旧数字 `7` 能反序列化为 `AfterHours`。
  - `mock` 字段为 `None` vs `Some(false)` 在两种 server 上的行为差异（A 是 `bool` 默认 false，TW 是 `bool` 默认 false）——确认 `None` 与 `Some(false)` 等效。

---

## 验收标准

- [ ] `AClient` 能用 `mock: true` 下单并读到 `fill_price` / `filled_at_ms`；`dry_run` 与 `mock` 同开在客户端即被拒。
- [ ] `AClient` / `TwClient` 各自能完成「初始化模拟账户 → 查账户 → mock 下单 → 查单」闭环。
- [ ] `Box<dyn BrokerClient>` 用同一份代码对 A/TW 两个 server 跑通 mock 全流程。
- [ ] 台股可通过本库提交 `ap_code = ODD_LOT / AFTER_HOURS` 订单。
- [ ] `BrokerEvent` 能区分 `mock.account_changed` 与 `ws.lagged`，不再降级为 `Unknown`。
- [ ] 不携带任何新字段时，所有既有请求的 wire 字节与 0.3.0 完全一致（有测试断言）。
- [ ] 全部现有测试零改动通过；CI（fmt / clippy / test / doc）全绿。
- [ ] `docs/a-client-api.md` 与 `docs/tw-client-api.md` 同步更新，`CHANGELOG.md` 记为 0.4.0。

---

## 兼容性检查

- **是否影响现有行为**：
  - 不携带新字段时行为**完全不变**（`mock`/`ap_code` 均为 `Option` 且 `skip_serializing_if = "Option::is_none"`）。写入路径、重试策略、错误映射一律不动。
  - **唯一行为变化**：`AClient::submit_order` 增加 `mock && dry_run` 的本地前置校验（服务端本来就会 400，只是提前到客户端）。

- **是否需要兼容旧接口/旧数据**：
  - 新类型全部 `#[serde(default)]`，旧的 HTTP 响应、旧的 WS 事件、旧的 JSON 快照都能继续解析。
  - `AEvent` / `BrokerEvent` 新变体是**新增**，原有 `match` 若带 `_ =>` 分支不受影响；若调用方做了穷举 `match`，会有编译错误——这是可接受的显式失败，但必须在 CHANGELOG 的 `Changed` 段声明。
  - `ap_code` 需要双向兼容：既能发字符串，也能收旧数字 `0/2/4/7`。

- **是否存在 break userspace 风险**：
  - **有，且主要在两处，必须显式处理**：
    1. **`types::OrderRequest` 是公共结构体、字段全 `pub`、没有 `Default`**。给它加字段会破坏任何用结构体字面量构造它的下游代码（本仓库自己的 `tests/unified.rs:19-35` 就是这种写法）。**缓解措施**：新增字段的同时为其 `derive(Default)`，本轮改动把仓库内所有字面量构造改为 builder 或 `..Default::default()`，并在 CHANGELOG 里把这一条列为 **breaking change**（即便版本号仍走 0.4.0 次版本，也要写明"下游若用结构体字面量构造 `OrderRequest` 需补 `..Default::default()`"）。
    2. **`BrokerClient` trait 新增方法**会破坏所有现有的 `impl BrokerClient for ...` 外部实现。**缓解措施**：为两个新方法提供默认实现（默认返回 `Err(Error::InvalidRequest("mock account is not supported"))`），把它降级为**非破坏性**变更。当前已知实现只有本仓库两个，但不能假设将来没有第三方实现。
  - 其余改动（新类型、新事件变体、新方法）均为纯增量，无 break 风险。
  - 版本策略：`0.3.0 → 0.4.0`，`CHANGELOG.md` 中 `OrderRequest` 加字段一条归入 `Changed` 并标注结构体字段变更，其余归入 `Added`。