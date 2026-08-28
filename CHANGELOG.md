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
