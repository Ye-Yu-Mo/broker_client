//! Strongly-typed models for the A-share (同花顺) server API.
//!
//! The A server documents fewer example payloads than the TW server. All
//! response models therefore use `#[serde(default)]` and keep unknown fields
//! in `extra` so server additions do not break parsing.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A query response that may have been served from a local cache snapshot.
///
/// The A server can downgrade a failed read to the most recent local snapshot
/// and mark the response with `from_cache` and `cached_at`. This type preserves
/// those markers; callers can inspect `from_cache` to decide how fresh the data
/// is.
#[derive(Debug, Clone, PartialEq)]
pub struct Cached<T> {
    /// The actual response payload (account, positions, orders, etc.).
    pub data: T,
    /// `true` when this response was served from a local cache snapshot.
    pub from_cache: bool,
    /// When the cache snapshot was taken, if the server sent it.
    pub cached_at: Option<String>,
}

impl<T> Cached<T> {
    /// Maps the payload while preserving cache metadata.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Cached<U> {
        Cached {
            data: f(self.data),
            from_cache: self.from_cache,
            cached_at: self.cached_at,
        }
    }
}

impl<'de, T> Deserialize<'de> for Cached<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut value = Value::deserialize(deserializer)?;
        let from_cache = value
            .get("from_cache")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let cached_at = value
            .get("cached_at")
            .and_then(Value::as_str)
            .map(str::to_owned);

        // Support both a bare response (`{ ...fields... }`) and a wrapped
        // response (`{ "data": {...}, "from_cache": true }` or
        // `{ "data": {...}, "cached_at": ... }` without `from_cache`).
        if value.get("data").is_some()
            && (value.get("from_cache").is_some() || value.get("cached_at").is_some())
        {
            let data_value = value
                .get("data")
                .cloned()
                .unwrap_or_else(|| Value::Object(Default::default()));
            let data = T::deserialize(data_value).map_err(serde::de::Error::custom)?;
            return Ok(Cached {
                data,
                from_cache,
                cached_at,
            });
        }

        if let Value::Object(map) = &mut value {
            map.remove("from_cache");
            map.remove("cached_at");
        }
        let data = T::deserialize(value).map_err(serde::de::Error::custom)?;
        Ok(Cached {
            data,
            from_cache,
            cached_at,
        })
    }
}

/// Funds / account summary returned by `GET /v1/account`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountFunds {
    /// Account identifier, if present.
    #[serde(default)]
    pub account: Option<String>,
    /// Total assets.
    #[serde(default)]
    pub total_asset: Option<f64>,
    /// Available cash.
    #[serde(default)]
    pub available: Option<f64>,
    /// Current market value of positions.
    #[serde(default)]
    pub market_value: Option<f64>,
    /// Frozen or reserved amount.
    #[serde(default)]
    pub frozen: Option<f64>,
    /// Cash balance.
    #[serde(default)]
    pub cash: Option<f64>,
    /// Currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// Last update time.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A stock position returned by `GET /v1/positions`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Position {
    /// Account that owns the position.
    #[serde(default)]
    pub account: Option<String>,
    /// Stock symbol/code, e.g. `512100`.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Stock name.
    #[serde(default)]
    pub name: Option<String>,
    /// Total quantity.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Tradable quantity.
    #[serde(default)]
    pub available_quantity: Option<f64>,
    /// Cost price.
    #[serde(default)]
    pub cost_price: Option<f64>,
    /// Last price.
    #[serde(default)]
    pub last_price: Option<f64>,
    /// Market price.
    #[serde(default)]
    pub market_price: Option<f64>,
    /// Market value.
    #[serde(default)]
    pub market_value: Option<f64>,
    /// Profit/loss amount.
    #[serde(default)]
    pub pnl: Option<f64>,
    /// Profit/loss ratio.
    #[serde(default)]
    pub pnl_ratio: Option<f64>,
    /// Last update time.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// An order record returned by the order list/detail endpoints.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Order {
    /// Client-generated idempotency key.
    #[serde(default)]
    pub client_order_id: Option<String>,
    /// Broker order number.
    #[serde(default)]
    pub order_no: Option<String>,
    /// Stock symbol/code.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Stock name.
    #[serde(default)]
    pub name: Option<String>,
    /// Side, e.g. `buy` or `sell`.
    #[serde(default)]
    pub side: Option<String>,
    /// Order price.
    #[serde(default)]
    pub price: Option<f64>,
    /// Order quantity.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Filled quantity.
    #[serde(default)]
    pub filled_quantity: Option<f64>,
    /// Order status/message text.
    #[serde(default)]
    pub status: Option<String>,
    /// Server-provided message, if any.
    #[serde(default)]
    pub message: Option<String>,
    /// Whether this was a dry-run submission.
    #[serde(default)]
    pub dry_run: Option<bool>,
    /// Creation time.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last update time.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A trade/fill record returned by `GET /v1/orders?type=trade`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Trade {
    /// Trade identifier.
    #[serde(default)]
    pub trade_id: Option<String>,
    /// Client order ID.
    #[serde(default)]
    pub client_order_id: Option<String>,
    /// Broker order number.
    #[serde(default)]
    pub order_no: Option<String>,
    /// Stock symbol/code.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Stock name.
    #[serde(default)]
    pub name: Option<String>,
    /// Side, e.g. `buy` or `sell`.
    #[serde(default)]
    pub side: Option<String>,
    /// Fill price.
    #[serde(default)]
    pub price: Option<f64>,
    /// Fill quantity.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Fill amount.
    #[serde(default)]
    pub amount: Option<f64>,
    /// Trade time.
    #[serde(default)]
    pub trade_time: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Profit and loss summary returned by `GET /v1/pnl`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Pnl {
    /// Total P&L.
    #[serde(default)]
    pub total_pnl: Option<f64>,
    /// Today's P&L.
    #[serde(default)]
    pub today_pnl: Option<f64>,
    /// Realized P&L.
    #[serde(default)]
    pub realized_pnl: Option<f64>,
    /// Unrealized P&L.
    #[serde(default)]
    pub unrealized_pnl: Option<f64>,
    /// Total assets, if included.
    #[serde(default)]
    pub total_asset: Option<f64>,
    /// Last update time.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A cash transaction / statement row returned by `GET /v1/account/transactions`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Transaction {
    /// Transaction identifier.
    #[serde(default)]
    pub transaction_id: Option<String>,
    /// Transaction type, e.g. `buy`, `sell`, `deposit`.
    #[serde(default)]
    pub transaction_type: Option<String>,
    /// Related symbol, when applicable.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Transaction amount.
    #[serde(default)]
    pub amount: Option<f64>,
    /// Balance after the transaction.
    #[serde(default)]
    pub balance: Option<f64>,
    /// Free-form remark.
    #[serde(default)]
    pub remark: Option<String>,
    /// Transaction time.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Structured health state returned by `GET /v1/health`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Health {
    /// Overall status.
    #[serde(default)]
    pub status: Option<String>,
    /// Whether the 同花顺 (THS) adapter is online.
    #[serde(default)]
    pub ths_online: Option<bool>,
    /// Whether the AX permission is available.
    #[serde(default)]
    pub ax_permission: Option<bool>,
    /// GUI queue size/depth.
    #[serde(default)]
    pub gui_queue: Option<f64>,
    /// Whether the GUI is busy.
    #[serde(default)]
    pub gui_busy: Option<bool>,
    /// Current GUI queue depth.
    #[serde(default)]
    pub gui_queue_depth: Option<f64>,
    /// Last refresh timestamp in milliseconds, when provided.
    #[serde(default)]
    pub last_refresh_at_ms: Option<i64>,
    /// Whether the last cancel operation succeeded.
    #[serde(default)]
    pub last_cancel_success: Option<bool>,
    /// Whether the last order operation succeeded.
    #[serde(default)]
    pub last_order_success: Option<bool>,
    /// Last refresh time.
    #[serde(default)]
    pub last_refresh: Option<String>,
    /// Last operation description.
    #[serde(default)]
    pub last_operation: Option<String>,
    /// Panic/circuit-breaker state. The live server returns an object, so this
    /// is kept as raw JSON to remain forward-compatible.
    #[serde(default)]
    pub panic: Option<Value>,
    /// Whether audit writing is available.
    #[serde(default)]
    pub audit_writable: Option<bool>,
    /// Server version, if present.
    #[serde(default)]
    pub version: Option<String>,
    /// Health response timestamp, if present.
    #[serde(default)]
    pub timestamp_ms: Option<i64>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Request body for `POST /v1/orders`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OrderRequest {
    /// Client-generated idempotency key.
    pub client_order_id: String,
    /// Stock symbol/code.
    pub symbol: String,
    /// Order side: `buy` or `sell`.
    pub side: String,
    /// Limit price.
    pub price: f64,
    /// Order quantity.
    pub quantity: i64,
    /// `true` only fills the form and does not click confirm.
    pub dry_run: bool,
}

impl OrderRequest {
    /// Creates a new order request.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client_order_id: impl Into<String>,
        symbol: impl Into<String>,
        side: impl Into<String>,
        price: f64,
        quantity: i64,
        dry_run: bool,
    ) -> Self {
        Self {
            client_order_id: client_order_id.into(),
            symbol: symbol.into(),
            side: side.into(),
            price,
            quantity,
            dry_run,
        }
    }
}

/// Request body for `POST /v1/orders/{client_order_id}/cancel`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CancelRequest {
    /// Optional cancellation reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CancelRequest {
    /// Creates an empty cancel request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a cancel request with a reason.
    pub fn with_reason(reason: impl Into<String>) -> Self {
        Self {
            reason: Some(reason.into()),
        }
    }
}

/// Request body for `POST /v1/orders/{client_order_id}/replace`.
///
/// The server requires at least one of `new_price` / `new_quantity`. The
/// builder methods make it easy to construct valid requests. Use
/// [`ReplaceRequest::validate`] before sending if you build a request with
/// neither field.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReplaceRequest {
    /// Must be `"replace"` per the documented API.
    pub action: String,
    /// Broker order number to replace.
    pub order_no: String,
    /// New price. At least one of this and `new_quantity` must be set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_price: Option<f64>,
    /// New quantity. At least one of this and `new_price` must be set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_quantity: Option<i64>,
    /// `true` only validates the request; no cancel/replace is performed.
    pub dry_run: bool,
}

impl ReplaceRequest {
    /// Creates a price-only replace request.
    pub fn price(order_no: impl Into<String>, new_price: f64, dry_run: bool) -> Self {
        Self {
            action: "replace".to_owned(),
            order_no: order_no.into(),
            new_price: Some(new_price),
            new_quantity: None,
            dry_run,
        }
    }

    /// Creates a quantity-only replace request.
    pub fn quantity(order_no: impl Into<String>, new_quantity: i64, dry_run: bool) -> Self {
        Self {
            action: "replace".to_owned(),
            order_no: order_no.into(),
            new_price: None,
            new_quantity: Some(new_quantity),
            dry_run,
        }
    }

    /// Creates a replace request with neither field set (for incremental
    /// builder use).
    pub fn builder(order_no: impl Into<String>, dry_run: bool) -> Self {
        Self {
            action: "replace".to_owned(),
            order_no: order_no.into(),
            new_price: None,
            new_quantity: None,
            dry_run,
        }
    }

    /// Sets the new price.
    pub fn with_price(mut self, new_price: f64) -> Self {
        self.new_price = Some(new_price);
        self
    }

    /// Sets the new quantity.
    pub fn with_quantity(mut self, new_quantity: i64) -> Self {
        self.new_quantity = Some(new_quantity);
        self
    }

    /// Returns `true` when at least one of `new_price` / `new_quantity` is set.
    pub fn is_valid(&self) -> bool {
        self.new_price.is_some() || self.new_quantity.is_some()
    }

    /// Validates that at least one of `new_price` / `new_quantity` is set.
    pub fn validate(&self) -> crate::Result<()> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(crate::Error::InvalidRequest(
                "replace request must set new_price and/or new_quantity".to_owned(),
            ))
        }
    }
}

/// Response for `POST /v1/refresh`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RefreshResponse {
    /// Refresh status.
    #[serde(default)]
    pub status: Option<String>,
    /// Server message.
    #[serde(default)]
    pub message: Option<String>,
    /// When the refresh was completed.
    #[serde(default)]
    pub refreshed_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Request body for `POST /v1/control/panic`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PanicRequest {
    /// Optional panic reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl PanicRequest {
    /// Creates a panic request without a reason.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a panic request with a reason.
    pub fn with_reason(reason: impl Into<String>) -> Self {
        Self {
            reason: Some(reason.into()),
        }
    }
}

/// WebSocket events pushed by the A-share server.
///
/// Every known variant keeps the raw `data` JSON and the optional
/// `timestamp_ms` so callers never lose server-side details.
#[derive(Debug, Clone, PartialEq)]
pub enum AEvent {
    /// `order.updated`
    OrderUpdated {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `position.changed`
    PositionChanged {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `account.changed`
    AccountChanged {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `account.balance_changed`
    AccountBalanceChanged {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `query.cache_hit`
    QueryCacheHit {
        /// Raw event payload (includes `from_cache` / `cached_at` when sent).
        data: Value,
        /// `true` when the query was served from cache.
        from_cache: bool,
        /// Cache snapshot time, if present.
        cached_at: Option<String>,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `replace.updated`
    ReplaceUpdated {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `order.no_mapping`
    OrderNoMapping {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `order.manual_review`
    OrderManualReview {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `risk.panic`
    RiskPanic {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `health.changed`
    HealthChanged {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// Any other event type.
    Unknown {
        /// Original `type` string.
        type_name: String,
        /// Raw event data.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
}

impl<'de> Deserialize<'de> for AEvent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let type_name = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let timestamp_ms = value.get("timestamp_ms").and_then(Value::as_i64);
        let data = value.get("data").cloned().unwrap_or(value.clone());

        let timestamp = timestamp_ms;
        Ok(match type_name.as_str() {
            "order.updated" => AEvent::OrderUpdated {
                data,
                timestamp_ms: timestamp,
            },
            "position.changed" => AEvent::PositionChanged {
                data,
                timestamp_ms: timestamp,
            },
            "account.changed" => AEvent::AccountChanged {
                data,
                timestamp_ms: timestamp,
            },
            "account.balance_changed" => AEvent::AccountBalanceChanged {
                data,
                timestamp_ms: timestamp,
            },
            "query.cache_hit" => AEvent::QueryCacheHit {
                from_cache: data
                    .get("from_cache")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                cached_at: data
                    .get("cached_at")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                data,
                timestamp_ms: timestamp,
            },
            "replace.updated" => AEvent::ReplaceUpdated {
                data,
                timestamp_ms: timestamp,
            },
            "order.no_mapping" => AEvent::OrderNoMapping {
                data,
                timestamp_ms: timestamp,
            },
            "order.manual_review" => AEvent::OrderManualReview {
                data,
                timestamp_ms: timestamp,
            },
            "risk.panic" => AEvent::RiskPanic {
                data,
                timestamp_ms: timestamp,
            },
            "health.changed" => AEvent::HealthChanged {
                data,
                timestamp_ms: timestamp,
            },
            _ => AEvent::Unknown {
                type_name,
                data,
                timestamp_ms: timestamp,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn account_funds_parses_with_unknown_fields() {
        let value: AccountFunds = serde_json::from_value(json!({
            "account": "A1",
            "total_asset": 12345.67,
            "available": 1000.0,
            "future_field": {"x": 1}
        }))
        .unwrap();
        assert_eq!(value.account.as_deref(), Some("A1"));
        assert_eq!(value.total_asset, Some(12345.67));
        assert_eq!(value.extra["future_field"]["x"], 1);
    }

    #[test]
    fn position_order_trade_pnl_transaction_health_parse() {
        let position: Position = serde_json::from_value(json!({
            "symbol": "512100",
            "quantity": 100,
            "extra_field": 1
        }))
        .unwrap();
        assert_eq!(position.symbol.as_deref(), Some("512100"));
        assert_eq!(position.extra["extra_field"], 1);

        let order: Order = serde_json::from_value(json!({
            "client_order_id": "C1",
            "status": "Confirmed",
            "price": 3.305
        }))
        .unwrap();
        assert_eq!(order.status.as_deref(), Some("Confirmed"));

        let trade: Trade = serde_json::from_value(json!({
            "trade_id": "T1",
            "symbol": "512100",
            "quantity": 100
        }))
        .unwrap();
        assert_eq!(trade.trade_id.as_deref(), Some("T1"));

        let pnl: Pnl = serde_json::from_value(json!({
            "total_pnl": 10.5,
            "today_pnl": 2.5
        }))
        .unwrap();
        assert_eq!(pnl.total_pnl, Some(10.5));

        let tx: Transaction = serde_json::from_value(json!({
            "transaction_id": "X1",
            "amount": -100.0
        }))
        .unwrap();
        assert_eq!(tx.amount, Some(-100.0));

        let health: Health = serde_json::from_value(json!({
            "status": "ok",
            "ths_online": true,
            "ax_permission": true,
            "gui_queue": 0,
            "audit_writable": true
        }))
        .unwrap();
        assert_eq!(health.status.as_deref(), Some("ok"));
        assert_eq!(health.ths_online, Some(true));
    }

    #[test]
    fn cached_parses_direct_and_wrapped_responses() {
        let direct: Cached<AccountFunds> = serde_json::from_value(json!({
            "account": "A1",
            "total_asset": 1.0,
            "from_cache": true,
            "cached_at": "2026-08-28T10:00:00Z"
        }))
        .unwrap();
        assert!(direct.from_cache);
        assert_eq!(direct.cached_at.as_deref(), Some("2026-08-28T10:00:00Z"));
        assert_eq!(direct.data.account.as_deref(), Some("A1"));

        let wrapped: Cached<AccountFunds> = serde_json::from_value(json!({
            "data": {"account": "A2"},
            "from_cache": false
        }))
        .unwrap();
        assert!(!wrapped.from_cache);
        assert_eq!(wrapped.data.account.as_deref(), Some("A2"));

        let wrapped_cached_at: Cached<AccountFunds> = serde_json::from_value(json!({
            "data": {"account": "A3"},
            "cached_at": "2026-08-28T10:00:00Z"
        }))
        .unwrap();
        assert!(!wrapped_cached_at.from_cache);
        assert_eq!(
            wrapped_cached_at.cached_at.as_deref(),
            Some("2026-08-28T10:00:00Z")
        );
        assert_eq!(wrapped_cached_at.data.account.as_deref(), Some("A3"));
    }

    #[test]
    fn cached_defaults_from_cache_to_false() {
        let value: Cached<AccountFunds> = serde_json::from_value(json!({"account": "A1"})).unwrap();
        assert!(!value.from_cache);
        assert_eq!(value.cached_at, None);
    }

    #[test]
    fn cached_empty_list_parses() {
        let value: Cached<Vec<Position>> = serde_json::from_value(json!([])).unwrap();
        assert!(value.data.is_empty());
        assert!(!value.from_cache);
        assert_eq!(value.cached_at, None);
    }

    #[test]
    fn order_request_serializes_dry_run_differences() {
        let real = OrderRequest::new("C1", "512100", "buy", 3.305, 100, false);
        let dry = OrderRequest::new("C2", "512100", "buy", 3.305, 100, true);
        let real_json = serde_json::to_value(&real).unwrap();
        let dry_json = serde_json::to_value(&dry).unwrap();
        assert_eq!(real_json["dry_run"], false);
        assert_eq!(dry_json["dry_run"], true);
        assert_eq!(real_json["client_order_id"], "C1");
        assert_eq!(real_json["symbol"], "512100");
        assert_eq!(real_json["side"], "buy");
        assert_eq!(real_json["price"], 3.305);
        assert_eq!(real_json["quantity"], 100);
    }

    #[test]
    fn replace_request_requires_at_least_one_field() {
        let req = ReplaceRequest::price("H1", 3.3, true);
        assert!(req.is_valid());
        assert!(req.validate().is_ok());
        assert_eq!(serde_json::to_value(&req).unwrap()["new_price"], 3.3);

        let qty = ReplaceRequest::quantity("H1", 200, false);
        assert!(qty.is_valid());
        assert_eq!(serde_json::to_value(&qty).unwrap()["new_quantity"], 200);

        let empty = ReplaceRequest::builder("H1", false);
        assert!(!empty.is_valid());
        assert!(empty.validate().is_err());

        let both = ReplaceRequest::builder("H1", false)
            .with_price(3.3)
            .with_quantity(200);
        assert!(both.is_valid());
    }

    #[test]
    fn cancel_and_panic_requests_serialize_optional_fields() {
        let cancel = CancelRequest::with_reason("manual");
        let json = serde_json::to_value(cancel).unwrap();
        assert_eq!(json["reason"], "manual");

        let empty_cancel = CancelRequest::new();
        let json = serde_json::to_value(empty_cancel).unwrap();
        assert!(json.get("reason").is_none());

        let panic = PanicRequest::with_reason("发现异常");
        let json = serde_json::to_value(panic).unwrap();
        assert_eq!(json["reason"], "发现异常");

        let empty_panic = PanicRequest::new();
        let json = serde_json::to_value(empty_panic).unwrap();
        assert!(json.get("reason").is_none());
    }

    #[test]
    fn a_event_parses_all_documented_types() {
        let cases = [
            ("order.updated", "OrderUpdated"),
            ("position.changed", "PositionChanged"),
            ("account.changed", "AccountChanged"),
            ("account.balance_changed", "AccountBalanceChanged"),
            ("query.cache_hit", "QueryCacheHit"),
            ("replace.updated", "ReplaceUpdated"),
            ("order.no_mapping", "OrderNoMapping"),
            ("order.manual_review", "OrderManualReview"),
            ("risk.panic", "RiskPanic"),
            ("health.changed", "HealthChanged"),
        ];

        for (type_name, variant) in cases {
            let event: AEvent = serde_json::from_value(json!({
                "type": type_name,
                "timestamp_ms": 1730000000000_i64,
                "data": {"value": 1}
            }))
            .unwrap();
            let name = match &event {
                AEvent::OrderUpdated { .. } => "OrderUpdated",
                AEvent::PositionChanged { .. } => "PositionChanged",
                AEvent::AccountChanged { .. } => "AccountChanged",
                AEvent::AccountBalanceChanged { .. } => "AccountBalanceChanged",
                AEvent::QueryCacheHit { .. } => "QueryCacheHit",
                AEvent::ReplaceUpdated { .. } => "ReplaceUpdated",
                AEvent::OrderNoMapping { .. } => "OrderNoMapping",
                AEvent::OrderManualReview { .. } => "OrderManualReview",
                AEvent::RiskPanic { .. } => "RiskPanic",
                AEvent::HealthChanged { .. } => "HealthChanged",
                AEvent::Unknown { .. } => "Unknown",
            };
            assert_eq!(name, variant);

            if let AEvent::QueryCacheHit {
                data,
                timestamp_ms,
                from_cache,
                ..
            } = &event
            {
                assert_eq!(data["value"], 1);
                assert_eq!(*timestamp_ms, Some(1730000000000_i64));
                assert!(!from_cache);
            } else {
                assert!(!matches!(&event, AEvent::Unknown { .. }));
            }
        }
    }

    #[test]
    fn a_event_unknown_preserves_type_data_and_timestamp() {
        let event: AEvent = serde_json::from_value(json!({
            "type": "future.event",
            "timestamp_ms": 42,
            "data": {"x": 1}
        }))
        .unwrap();
        match event {
            AEvent::Unknown {
                type_name,
                data,
                timestamp_ms,
            } => {
                assert_eq!(type_name, "future.event");
                assert_eq!(data["x"], 1);
                assert_eq!(timestamp_ms, Some(42));
            }
            _ => panic!("expected unknown"),
        }
    }

    #[test]
    fn a_event_query_cache_hit_preserves_cache_markers() {
        let event: AEvent = serde_json::from_value(json!({
            "type": "query.cache_hit",
            "timestamp_ms": 7,
            "data": {
                "from_cache": true,
                "cached_at": "2026-01-01T00:00:00Z",
                "account": "A1"
            }
        }))
        .unwrap();
        match event {
            AEvent::QueryCacheHit {
                data,
                from_cache,
                cached_at,
                timestamp_ms,
            } => {
                assert!(from_cache);
                assert_eq!(cached_at.as_deref(), Some("2026-01-01T00:00:00Z"));
                assert_eq!(data["account"], "A1");
                assert_eq!(timestamp_ms, Some(7));
            }
            _ => panic!("expected query cache hit"),
        }
    }

    #[test]
    fn a_event_without_timestamp_still_parses() {
        let event: AEvent = serde_json::from_value(json!({
            "type": "order.updated",
            "data": {"status": "FILLED"}
        }))
        .unwrap();
        match event {
            AEvent::OrderUpdated { timestamp_ms, data } => {
                assert_eq!(timestamp_ms, None);
                assert_eq!(data["status"], "FILLED");
            }
            _ => panic!("expected order updated"),
        }
    }

    #[test]
    fn a_event_malformed_json_is_rejected_by_strict_deserialize() {
        // AEvent's Deserialize is strict; the WebSocket layer converts malformed
        // text into an Unknown event so the stream never panics.
        assert!(serde_json::from_str::<AEvent>("not json").is_err());
    }
}
