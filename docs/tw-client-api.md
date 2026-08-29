# Client API 文档

本文档面向接入 stock-broker-tw-server 的客户端开发者，覆盖 HTTP API 与 WebSocket 推送。

- 服务默认地址：`http://127.0.0.1:8000`
- 默认认证：`Authorization: Bearer <api_token>`
- 如果服务端未配置 `api_token`，则不需要认证头。

## 1. 通用说明

### 1.1 请求头

| Header | 必填 | 说明 |
|---|---|---|
| `Authorization` | 视配置 | `Bearer <api_token>` |
| `Content-Type` | POST 必填 | `application/json` |
| `X-Request-ID` | 可选 | 请求追踪 ID，会写入审计日志 |

### 1.2 响应格式

成功响应统一为：

```json
{
  "code": 0,
  "message": "ok",
  "data": { }
}
```

失败响应使用 FastAPI/HTTPException 格式：

```json
{
  "detail": {
    "code": "ERROR_CODE",
    "message": "错误说明",
    "detail": { }
  }
}
```

### 1.3 常见错误码

| HTTP | code | 含义 |
|---|---|---|
| 401 | `UNAUTHORIZED` | 缺少或错误的 token |
| 400 | `INVALID_REQUEST` / `INVALID_DATE` / `MAX_PER_REQUEST_EXCEEDED` 等 | 请求参数错误 |
| 404 | `ORDER_NOT_FOUND` | 订单不存在 |
| 409 | `IDEMPOTENCY_CONFLICT` | `client_order_id` 已被不同操作使用 |
| 429 | `RATE_LIMITED` | 触发限流 |
| 502 | `QUERY_ERROR` / `SUBSCRIBE_FAILED` / `ORDER_REJECTED` 等 | 元大接口调用失败 |
| 503 | `CIRCUIT_OPEN` | 熔断开启，写接口暂时不可用 |

## 2. 健康与监控

### 2.1 健康检查

```
GET /health
```

无需认证。示例响应：

```json
{
  "status": "ok",
  "adapter_ready": true,
  "login_status": true,
  "event_queue_size": 0,
  "audit_enabled": true,
  "audit_file": null,
  "version": "0.1.0",
  "environment": "UAT",
  "panic": false,
  "circuit_breaker_open": false,
  "circuit_breaker": {
    "name": "yuanta",
    "state": "closed",
    "is_open": false,
    "consecutive_failures": 0,
    "failure_threshold": 5,
    "cooldown_seconds": 30.0,
    "last_error": null,
    "last_failure_at": null,
    "rejections": 0
  },
  "last_failure": null,
  "last_recovery": {
    "status": "ok",
    "unfinished_before": 0,
    "unfinished_after": 0,
    "reconciled": true,
    "unresolved_orders": 0,
    "unresolved_stock_orders": 0
  }
}
```

### 2.2 Prometheus 指标

```
GET /metrics
```

无需认证。客户端一般不需要调用，运维可通过该端点观察：

- `http_requests_total`
- `rate_limited_total`
- `circuit_breaker_state`
- `circuit_breaker_opens_total`
- `circuit_breaker_rejections_total`

## 3. 会话管理

### 3.1 登录

```
POST /api/v1/session/login
```

请求体可选；缺省使用服务端配置的账号。

```json
{
  "account": "S98875005091",
  "password": "your-password",
  "pfx_path": "/path/to/cert.pfx",
  "pfx_pass": "your-pfx-password"
}
```

响应：

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "login": {
      "login_list": [
        {
          "account": "S98875005091",
          "name": "測試用戶",
          "investor_id": "A123456789"
        }
      ]
    },
    "account": "S98875005091",
    "name": "測試用戶",
    "investor_id": "A123456789"
  }
}
```

### 3.2 登出

```
POST /api/v1/session/logout
```

### 3.3 查询会话状态

```
GET /api/v1/session/status
```

## 4. 账户与查询

所有查询接口均可选传 `account`；缺省使用服务端默认账户。

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/v1/positions` | 库存/持仓 |
| GET | `/api/v1/account/balance` | 银行余额 |
| GET | `/api/v1/account/settlement` | 交割金额 |
| GET | `/api/v1/pnl/unrealized?market_type=TWSE&stk_code=2330` | 未实现损益 |
| GET | `/api/v1/pnl/realized?start_date=2026/01/01&end_date=2026/01/31` | 已实现损益 |
| GET | `/api/v1/pnl/reversal` | 反向损益，`re_gain_loss` 可传 JSON 字符串 |
| GET | `/api/v1/reports/real` | 实时回报 |
| GET | `/api/v1/reports/real-merge` | 合并回报 |
| GET | `/api/v1/reports/order-trade?notshow_cancel=false` | 委托/成交回报 |

示例：

```bash
curl -H "Authorization: Bearer test-token" \
  "http://127.0.0.1:8000/api/v1/positions?account=S98875005091"
```

## 5. 行情

### 5.1 订阅行情

```
POST /api/v1/quotes/subscribe
```

请求体：

```json
{
  "type": "five_tick",
  "symbols": ["2330", "2885"],
  "account": "S98875005091",
  "market_type": "TWSE",
  "index_flag": 7
}
```

`type` 支持：

| type | 对应元大订阅 |
|---|---|
| `watchlist` | SubscribeWatchlist |
| `watchlist_all` | SubscribeWatchlistAll |
| `five_tick` | SubscribeFiveTickA |
| `stock_tick` | SubscribeStockTick |
| `market_info` | SubscribeMarketInformation |
| `stock_info` | SubscribeStockInformation |

`index_flag` 主要用于 `watchlist`，相同股票不同 `index_flag` 会作为不同订阅保存。

### 5.2 取消订阅

```
POST /api/v1/quotes/unsubscribe
```

请求体与订阅相同。

### 5.3 查看已订阅

```
GET /api/v1/quotes/subscribed?source=local
GET /api/v1/quotes/subscribed?source=broker
GET /api/v1/quotes/subscribed?source=both
```

- `source=local`：默认，返回本地 SQLite 订阅清单。
- `source=broker`：调用元大 `GetQuoteList` 返回券商端实际已订阅清单。
- `source=both`：同时返回本地与券商端清单。

### 5.4 行情查询

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/v1/quotes/snapshot?stk_code=2330&market_type=TWSE` | 报价快照 |
| GET | `/api/v1/quotes/ticks?stk_code=2330&market_type=TWSE&last_count=20` | 分时明细 |
| GET | `/api/v1/quotes/classify-price?stk_code=2330&market_type=TWSE` | 分价量 |
| GET | `/api/v1/quotes/kline?stk_code=2330&start_date=2026/01/01&end_date=2026/01/31` | K 线 |
| GET | `/api/v1/stocks/info?stk_code=2330&market_type=TWSE` | 个股资讯 |

## 6. 交易

### 6.1 委托下单 / 撤单 / 改价 / 改量

```
POST /api/v1/orders/stock
```

统一入口，通过 `action` 区分操作。

#### 新单

```json
{
  "client_order_id": "C001",
  "action": "new",
  "account": "S98875005091",
  "stk_code": "2330",
  "side": "B",
  "price": 500.0,
  "quantity": 10,
  "time_in_force": "ROD",
  "price_flag": "LIMIT"
}
```

#### 撤单

```json
{
  "client_order_id": "C002",
  "action": "cancel",
  "account": "S98875005091",
  "order_no": "H00001",
  "trade_date": "2026/08/28",
  "stk_code": "2330",
  "side": "B"
}
```

#### 改价

```json
{
  "client_order_id": "C003",
  "action": "replace",
  "account": "S98875005091",
  "order_no": "H00001",
  "stk_code": "2330",
  "side": "B",
  "new_price": 510.0
}
```

#### 改量

```json
{
  "client_order_id": "C003",
  "action": "replace",
  "account": "S98875005091",
  "order_no": "H00001",
  "stk_code": "2330",
  "side": "B",
  "new_quantity": 20
}
```

> 注意：`replace` 不允许同时传 `new_price` 和 `new_quantity`，否则返回 `REPLACE_BOTH_FIELDS_UNSUPPORTED`。

### 6.2 幂等性

`client_order_id` 是幂等键：

- 同一 `client_order_id` 相同 `action` 重复提交不会重复送单。
- 同一 `client_order_id` 不同 `action` 返回 `IDEMPOTENCY_CONFLICT`。

### 6.3 订单状态

| 状态 | 含义 |
|---|---|
| `PENDING` | 已接收，等待发送 |
| `SUBMITTED` | 已发送券商 |
| `ACCEPTED` | 已接受 |
| `PARTIALLY_FILLED` | 部分成交 |
| `FILLED` | 全部成交 |
| `CANCELLED` | 已取消 |
| `REJECTED` | 被拒绝 |
| `FAILED` | 本地失败 |
| `NEED_MANUAL_REVIEW` | 需要人工确认 |

### 6.4 查询订单

```
GET /api/v1/orders?account=S98875005091&status=ACCEPTED
GET /api/v1/orders/{client_order_id}
```

## 7. 风控与运维控制

### 7.1 手动 Panic

```
POST /api/v1/control/panic
```

开启后所有下单/撤单/改单会被风控拒绝。

### 7.2 恢复

```
POST /api/v1/control/resume
```

关闭 Panic 并手动复位熔断器。

## 8. 恢复与人工确认

### 8.1 查看未解决订单

```
GET /api/v1/recovery/unresolved
```

响应 `data` 为数组，每项包含 `source`，取值 `orders`（M3 旧表）或 `stock_orders`（M4 新表）。

### 8.2 人工确认订单

```
POST /api/v1/recovery/{client_order_id}/resolve
```

请求体：

```json
{
  "status": "FILLED",
  "order_no": "H00001",
  "trade_date": "2026/08/28",
  "source": "stock_orders",
  "note": "人工确认"
}
```

- `source` 为 `orders` 时，必须提供 `order_no`。
- `source` 省略时优先按 M4 `stock_orders` 处理；若找不到，会尝试按 `order_no` 或路径参数作为 M3 旧订单号处理。

## 9. WebSocket

### 9.1 连接

```
WS /ws
```

认证使用请求头 `Authorization: Bearer <api_token>`。

连接成功后先收到：

```json
{
  "type": "welcome",
  "message": "connected"
}
```

### 9.2 接收事件

服务端会推送以下事件：

| type | 说明 |
|---|---|
| `Login` | 登录事件 |
| `RR_RealReport` | 实时回报原始事件 |
| `RR_RealReportMerge` | 合并回报原始事件 |
| `real_report` / `real_report_merge` | 处理后的回报事件 |
| `order.updated` | 订单状态更新 |
| `SubscribeWatchlist` / `SubscribeFiveTickA` 等 | 订阅原始行情事件 |
| `quote.updated` | 统一行情推送 |
| `heartbeat` | 每 30 秒心跳 |

示例回报推送：

```json
{
  "type": "order.updated",
  "data": {
    "client_order_id": "C001",
    "status": "FILLED",
    "order_no": "H00001",
    "trade_date": "2026/08/28",
    "request": { },
    "data": { },
    "last_error": null
  }
}
```

## 10. 快速示例

### 10.1 登录后查库存

```bash
TOKEN=test-token
BASE=http://127.0.0.1:8000

curl -X POST "$BASE/api/v1/session/login" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'

curl "$BASE/api/v1/positions?account=S98875005091" \
  -H "Authorization: Bearer $TOKEN"
```

### 10.2 下一笔限价买单

```bash
curl -X POST "$BASE/api/v1/orders/stock" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "client_order_id": "C001",
    "action": "new",
    "account": "S98875005091",
    "stk_code": "2330",
    "side": "B",
    "price": 500.0,
    "quantity": 10
  }'
```

### 10.3 订阅五档行情并监听 WebSocket

```bash
curl -X POST "$BASE/api/v1/quotes/subscribe" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "type": "five_tick",
    "symbols": ["2330"]
  }'
```

```bash
# 使用 wscat 或其他 WebSocket 客户端
wscat -c "ws://127.0.0.1:8000/ws" -H "Authorization: Bearer test-token"
```
