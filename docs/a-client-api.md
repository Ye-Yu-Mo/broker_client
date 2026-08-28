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

`dry_run=true` 只填表单不点确认。返回订单状态和消息。

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
