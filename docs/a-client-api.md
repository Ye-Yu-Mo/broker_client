# HTTP / WebSocket API

所有需要鉴权的接口通过请求头传递：

```http
Authorization: Bearer <token>
# 或
X-Auth-Token: <token>
```

## 版本

- 新接口统一使用 `/v1` 前缀，例如 `GET /v1/account`。
- 旧路径（`/account`、`/orders` 等）在过渡期继续可用，但建议新调用方迁移到 `/v1`。
- `/v1` 错误统一返回：
  ```json
  { "code": "error", "message": "...", "detail": {} }
  ```

## 只读接口

- `GET /v1/account`：资金账户，返回 `AccountFunds`
- `GET /v1/positions`：持仓列表；会自动切换到持仓面板后刷新；每项包含 `today_qty`（今仓）和 `yesterday_qty`（昨仓）
- `GET /v1/orders?type=order`：委托列表；会自动切换到委托面板后刷新
- `GET /v1/orders?type=trade`：成交列表；会自动切换到成交面板后刷新
- `GET /v1/orders?status=Confirmed`：本地订单记录按状态筛选
- `GET /v1/orders/{client_order_id}`：单笔本地订单记录
- `GET /v1/pnl`：当日盈亏/总盈亏
- `GET /v1/account/transactions`：资金流水/对账单
- `GET /v1/health`：结构化健康状态，包含同花顺在线、AX 权限、GUI 队列、最近刷新、最近操作、panic、审计可写性
- `GET /v1/metrics`：Prometheus 文本格式指标

查询接口支持限流；超限返回 `429`。查询失败且本地有最近快照时，会降级返回缓存并在响应中标记 `from_cache: true` 与 `cached_at`。

## 写接口

### `POST /v1/refresh`

刷新同花顺账户快照并持久化。

### `POST /v1/notify/test`

发送飞书测试报警，便于验证 webhook 配置。无需请求体。

### `POST /v1/orders`

提交订单。请求：

```json
{
  "client_order_id": "unique-id",
  "symbol": "512100",
  "side": "buy",
  "price": 3.305,
  "quantity": 100,
  "dry_run": false
}
```

`dry_run=true` 只填表单不点确认。

`mock=true` 不提交真实委托：服务端向同花顺交易界面输入证券代码，读取界面上的买一/卖一后模拟全量成交（买入用卖一价，卖出用买一价），并结算到服务端的模拟账户。`mock` 与 `dry_run` **互斥**，同时为 `true` 时服务端返回 `400`；客户端在 `AClient::submit_order` 里就提前拒绝，不发请求。

请求体的 `mock` 字段在 `false` 时**不发送**（服务端默认 `false`），因此不启用 mock 的调用方请求体与旧版本逐字节一致。

返回订单状态和消息。mock 成交的响应额外带：

| 字段 | 含义 |
|---|---|
| `mock` | `true` 表示这笔是模拟成交 |
| `contract_id` | 模拟合同编号，形如 `MOCK-<ms>-<client_order_id>` |
| `fill_price` | 模拟成交价 |
| `filled_quantity` | 模拟成交量 |
| `filled_at_ms` | 模拟成交时间（epoch 毫秒） |

普通真实下单与 dry-run 响应里 `fill_price` / `filled_at_ms` 为 `null`。

### `POST /v1/orders/{client_order_id}/cancel`

撤单。请求体可选：

```json
{ "reason": "manual" }
```

### `POST /v1/orders/{client_order_id}/replace`

改单（改量/改价）。请求：

```json
{
  "action": "replace",
  "order_no": "12345",
  "new_price": 3.30,
  "new_quantity": 200,
  "dry_run": false
}
```

`new_price` 与 `new_quantity` 至少提供一个。当前同花顺自动化流程不支持直接“改单”，接口按“先撤原委托，再按新价格/新数量重新下单”实现；`dry_run=true` 只做请求校验，不实际撤单/下单。

## 熔断接口

### `POST /v1/control/panic`

触发熔断。所有新下单/撤单/改单被拒绝，只读接口仍可用。请求体可选：

```json
{ "reason": "发现异常" }
```

### `POST /v1/control/resume`

解除熔断，恢复交易写操作。

## 模拟账户（mock）

服务端只保留**一个**模拟账户，与同花顺真实账户完全隔离：`mock=true` 的订单和这两个接口都只碰模拟账户，不读取也不修改真实账户快照。模拟账户持久化在服务端，重启后仍在。

### `POST /v1/mock/init_account`

初始化模拟账户。请求：

```json
{
  "cash": 100000,
  "positions": [
    {
      "symbol": "518850",
      "quantity": 1000,
      "available_quantity": 1000,
      "average_cost": 9.0
    }
  ],
  "reset": false
}
```

`positions` 与 `reset` 可省略。已有模拟账户时默认返回 `409`；只有显式 `reset=true` 才会覆盖现金和持仓。参数非法（现金为负、代码为空或重复、`available_quantity > quantity`）返回 `400`。

响应为模拟账户快照：

```json
{
  "cash": 100000.0,
  "positions": [
    {
      "symbol": "518850",
      "quantity": 1000.0,
      "available_quantity": 1000.0,
      "average_cost": 9.0
    }
  ],
  "created_at_ms": 1787910698040,
  "updated_at_ms": 1787910698040
}
```

初始化成功会推送 `mock.account_changed` 事件（见下方「[WebSocket](#websocket)」的事件清单）。

### `GET /v1/mock/account`

读取当前模拟账户，响应结构同上。尚未初始化时返回 `404`。

客户端侧对应：

- `AClient::init_mock_account(&InitMockAccountRequest)` → `POST /v1/mock/init_account`
- `AClient::mock_account()` → `GET /v1/mock/account`

两个接口的错误都会映射为 `Error::Api`（`code = "error"`），不会退化成看不到服务端消息的传输错误。

## WebSocket

`GET /v1/ws` 建立长连接，服务端推送事件：

```json
{ "type": "order.updated", "timestamp_ms": 1730000000000, "data": { ... } }
```

事件类型：

- `order.updated`：下单/撤单/状态变化
- `position.changed`：持仓变化
- `account.changed`：刷新后资金/账户变化
- `account.balance_changed`：余额变化
- `query.cache_hit`：查询降级返回缓存
- `replace.updated`：改单状态变化
- `order.no_mapping`：本地订单未找到合同编号映射
- `order.manual_review`：订单需要人工介入
- `risk.panic`：手动或自动熔断状态变化
- `health.changed`：健康状态变化
- `mock.account_changed`：模拟账户变化（初始化模拟账户、mock 成交结算后推送）
- `ws.lagged`：广播通道消费滞后，部分事件已被丢弃；`data.skipped` 为跳过的事件数量

> `ws.lagged` 意味着**订阅方已经漏事件了**，不是心跳。收到后应重新拉取账户/持仓/委托快照，而不是继续依赖增量推送。
