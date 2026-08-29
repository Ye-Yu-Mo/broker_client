//! Unified type model shared by every broker implementation.
//!
//! The A-share and Taiwan clients historically maintained separate models. This
//! module defines the common super-set used by [`crate::client::BrokerClient`]:
//! A/TW specific fields are preserved as `Option<T>` so no server-specific
//! information is lost.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unified order action.
///
/// This is the same enum historically exposed by the TW client; it is kept here
/// so the unified request type can represent new/cancel/replace uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderAction {
    /// New order.
    New,
    /// Cancel order.
    Cancel,
    /// Replace order (price or quantity).
    Replace,
}

/// Unified order request.
///
/// The type is a superset of both the A-share and TW order request shapes.
/// Common fields are `client_order_id`, `symbol`, `side`, `price`, and
/// `quantity`; server-specific fields are optional.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OrderRequest {
    /// Client-generated idempotency key.
    pub client_order_id: String,
    /// Order action. TW clients use this directly; A clients default to `New`.
    pub action: OrderAction,
    /// Account number (TW). A-share unified requests set this to an empty
    /// string; the A client ignores it.
    pub account: String,
    /// Stock code (TW nomenclature). A-share unified requests mirror `symbol`
    /// into this field so the historical TW public field type stays intact.
    pub stk_code: String,
    /// Common symbol/code (A-share and unified callers).
    ///
    /// Kept out of serialization so the historical TW request body remains
    /// byte-compatible (`stk_code` is the wire field for TW).
    #[serde(default, skip_serializing)]
    pub symbol: Option<String>,
    /// Order side: `buy`/`sell` for A, `B`/`S` for TW.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    /// Limit price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    /// Order quantity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
    /// Time in force (TW), e.g. `ROD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<String>,
    /// Price flag (TW), e.g. `LIMIT`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_flag: Option<String>,
    /// Broker order number (TW cancel/replace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_no: Option<String>,
    /// Trade date (TW cancel/replace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_date: Option<String>,
    /// New price (TW replace price).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_price: Option<f64>,
    /// New quantity (TW replace quantity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_quantity: Option<i64>,
    /// A-share dry-run flag.
    ///
    /// This is not serialized by the unified request type; the A-share client
    /// uses its dedicated [`crate::client::a::OrderRequest`] for HTTP bodies.
    #[serde(default, skip_serializing)]
    pub dry_run: Option<bool>,
}

impl OrderRequest {
    /// Constructs a TW-style new-order request.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client_order_id: impl Into<String>,
        account: impl Into<String>,
        stk_code: impl Into<String>,
        side: impl Into<String>,
        price: f64,
        quantity: i64,
        time_in_force: impl Into<String>,
        price_flag: impl Into<String>,
    ) -> Self {
        let stk_code = stk_code.into();
        Self {
            client_order_id: client_order_id.into(),
            action: OrderAction::New,
            account: account.into(),
            stk_code: stk_code.clone(),
            symbol: Some(stk_code),
            side: Some(side.into()),
            price: Some(price),
            quantity: Some(quantity),
            time_in_force: Some(time_in_force.into()),
            price_flag: Some(price_flag.into()),
            order_no: None,
            trade_date: None,
            new_price: None,
            new_quantity: None,
            dry_run: None,
        }
    }

    /// Constructs an A-share style new-order request using the unified fields.
    #[allow(clippy::too_many_arguments)]
    pub fn a_new(
        client_order_id: impl Into<String>,
        symbol: impl Into<String>,
        side: impl Into<String>,
        price: f64,
        quantity: i64,
        dry_run: bool,
    ) -> Self {
        let symbol = symbol.into();
        Self {
            client_order_id: client_order_id.into(),
            action: OrderAction::New,
            account: String::new(),
            stk_code: symbol.clone(),
            symbol: Some(symbol),
            side: Some(side.into()),
            price: Some(price),
            quantity: Some(quantity),
            time_in_force: None,
            price_flag: None,
            order_no: None,
            trade_date: None,
            new_price: None,
            new_quantity: None,
            dry_run: Some(dry_run),
        }
    }

    /// Constructs a TW cancel-order request.
    #[allow(clippy::too_many_arguments)]
    pub fn cancel(
        client_order_id: impl Into<String>,
        account: impl Into<String>,
        order_no: impl Into<String>,
        trade_date: impl Into<String>,
        stk_code: impl Into<String>,
        side: impl Into<String>,
    ) -> Self {
        let stk_code = stk_code.into();
        Self {
            client_order_id: client_order_id.into(),
            action: OrderAction::Cancel,
            account: account.into(),
            stk_code: stk_code.clone(),
            symbol: Some(stk_code),
            side: Some(side.into()),
            price: None,
            quantity: None,
            time_in_force: None,
            price_flag: None,
            order_no: Some(order_no.into()),
            trade_date: Some(trade_date.into()),
            new_price: None,
            new_quantity: None,
            dry_run: None,
        }
    }

    /// Constructs a TW replace-price request.
    #[allow(clippy::too_many_arguments)]
    pub fn replace_price(
        client_order_id: impl Into<String>,
        account: impl Into<String>,
        order_no: impl Into<String>,
        stk_code: impl Into<String>,
        side: impl Into<String>,
        new_price: f64,
    ) -> Self {
        let stk_code = stk_code.into();
        Self {
            client_order_id: client_order_id.into(),
            action: OrderAction::Replace,
            account: account.into(),
            stk_code: stk_code.clone(),
            symbol: Some(stk_code),
            side: Some(side.into()),
            price: None,
            quantity: None,
            time_in_force: None,
            price_flag: None,
            order_no: Some(order_no.into()),
            trade_date: None,
            new_price: Some(new_price),
            new_quantity: None,
            dry_run: None,
        }
    }

    /// Constructs a TW replace-quantity request.
    #[allow(clippy::too_many_arguments)]
    pub fn replace_quantity(
        client_order_id: impl Into<String>,
        account: impl Into<String>,
        order_no: impl Into<String>,
        stk_code: impl Into<String>,
        side: impl Into<String>,
        new_quantity: i64,
    ) -> Self {
        let stk_code = stk_code.into();
        Self {
            client_order_id: client_order_id.into(),
            action: OrderAction::Replace,
            account: account.into(),
            stk_code: stk_code.clone(),
            symbol: Some(stk_code),
            side: Some(side.into()),
            price: None,
            quantity: None,
            time_in_force: None,
            price_flag: None,
            order_no: Some(order_no.into()),
            trade_date: None,
            new_price: None,
            new_quantity: Some(new_quantity),
            dry_run: None,
        }
    }
}

/// Unified cancel request.
///
/// `AClient` only needs `client_order_id` (and optionally `reason`);
/// `TwClient` additionally needs the broker order number/account/trade date.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    /// Client-generated idempotency key.
    pub client_order_id: String,
    /// Optional cancellation reason (A-share).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Account number (TW).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Broker order number (TW).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_no: Option<String>,
    /// Trade date (TW).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_date: Option<String>,
    /// Stock code (TW).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stk_code: Option<String>,
    /// Unified symbol/code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Order side (TW).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
}

impl CancelOrderRequest {
    /// Creates an A-share style cancel request.
    pub fn new(client_order_id: impl Into<String>) -> Self {
        Self {
            client_order_id: client_order_id.into(),
            ..Self::default()
        }
    }

    /// Sets a cancellation reason.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Sets TW cancel fields.
    #[allow(clippy::too_many_arguments)]
    pub fn tw(
        client_order_id: impl Into<String>,
        account: impl Into<String>,
        order_no: impl Into<String>,
        trade_date: impl Into<String>,
        stk_code: impl Into<String>,
        side: impl Into<String>,
    ) -> Self {
        let stk_code = stk_code.into();
        Self {
            client_order_id: client_order_id.into(),
            reason: None,
            account: Some(account.into()),
            order_no: Some(order_no.into()),
            trade_date: Some(trade_date.into()),
            stk_code: Some(stk_code.clone()),
            symbol: Some(stk_code),
            side: Some(side.into()),
        }
    }
}

/// Unified order status/order record.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct OrderStatus {
    /// Client order ID.
    #[serde(default)]
    pub client_order_id: Option<String>,
    /// Current order status.
    #[serde(default)]
    pub status: Option<String>,
    /// Broker order number.
    #[serde(default)]
    pub order_no: Option<String>,
    /// Trade date.
    #[serde(default)]
    pub trade_date: Option<String>,
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Stock code (TW nomenclature).
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Unified symbol/code.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Side (`buy`/`sell` or `B`/`S`).
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
    /// Created time.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Updated time.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Stock name (A-share).
    #[serde(default)]
    pub name: Option<String>,
    /// Server-provided message (A-share).
    #[serde(default)]
    pub message: Option<String>,
    /// A-share dry-run flag.
    #[serde(default)]
    pub dry_run: Option<bool>,
    /// TW original request payload.
    #[serde(default)]
    pub request: Option<Value>,
    /// TW extra data payload.
    #[serde(default)]
    pub data: Option<Value>,
    /// TW last error payload.
    #[serde(default)]
    pub last_error: Option<Value>,
    /// Unknown response fields.
    #[serde(default, flatten)]
    pub extra: Value,
}

/// Unified position.
///
/// Contains both A-share (`symbol`, `name`, `today_qty`, `yesterday_qty`) and
/// TW (`stk_code`, `stock_name`, `market_type`) fields.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Position {
    /// Account that owns the position.
    #[serde(default)]
    pub account: Option<String>,
    /// Common symbol/code.
    #[serde(default)]
    pub symbol: Option<String>,
    /// TW stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Common stock name.
    #[serde(default)]
    pub name: Option<String>,
    /// TW stock name.
    #[serde(default)]
    pub stock_name: Option<String>,
    /// Market type (TW).
    #[serde(default)]
    pub market_type: Option<String>,
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
    /// Today's position (今仓, A-share).
    #[serde(default)]
    pub today_qty: Option<f64>,
    /// Yesterday's position (昨仓, A-share).
    #[serde(default)]
    pub yesterday_qty: Option<f64>,
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
    #[serde(default, flatten)]
    pub extra: Value,
}

/// Unified account summary.
///
/// This is a superset of the A-share `AccountFunds` and the TW `Balance`
/// response shapes.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Account {
    /// Account identifier.
    #[serde(default)]
    pub account: Option<String>,
    /// Display name, if provided.
    #[serde(default)]
    pub name: Option<String>,
    /// Currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// Total assets (A-share).
    #[serde(default)]
    pub total_asset: Option<f64>,
    /// Available cash (A-share).
    #[serde(default)]
    pub available: Option<f64>,
    /// Current market value.
    #[serde(default)]
    pub market_value: Option<f64>,
    /// Frozen or reserved amount.
    #[serde(default)]
    pub frozen: Option<f64>,
    /// Cash balance.
    #[serde(default)]
    pub cash: Option<f64>,
    /// Total bank balance (TW).
    #[serde(default)]
    pub total_balance: Option<f64>,
    /// Available balance (TW).
    #[serde(default)]
    pub available_balance: Option<f64>,
    /// Withdrawable balance (TW).
    #[serde(default)]
    pub withdrawable: Option<f64>,
    /// Last update time.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// A-share cache downgrade marker.
    #[serde(default)]
    pub from_cache: bool,
    /// A-share cache snapshot time.
    #[serde(default)]
    pub cached_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten)]
    pub extra: Value,
}

/// Unified health summary.
///
/// This type is intentionally not re-exported at the crate root as `Health`
/// because the historical TW `Health` type has a non-optional `status` field.
/// Trait users can refer to [`crate::types::Health`].
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Health {
    /// Overall status.
    #[serde(default)]
    pub status: Option<String>,
    /// Whether the THS adapter is online (A-share).
    #[serde(default)]
    pub ths_online: Option<bool>,
    /// Whether the AX permission is available (A-share).
    #[serde(default)]
    pub ax_permission: Option<bool>,
    /// GUI queue size/depth (A-share).
    #[serde(default)]
    pub gui_queue: Option<f64>,
    /// Whether the GUI is busy (A-share).
    #[serde(default)]
    pub gui_busy: Option<bool>,
    /// Current GUI queue depth (A-share).
    #[serde(default)]
    pub gui_queue_depth: Option<f64>,
    /// Last refresh timestamp (A-share).
    #[serde(default)]
    pub last_refresh_at_ms: Option<i64>,
    /// Whether the last cancel succeeded (A-share).
    #[serde(default)]
    pub last_cancel_success: Option<bool>,
    /// Whether the last order succeeded (A-share).
    #[serde(default)]
    pub last_order_success: Option<bool>,
    /// Last refresh time (A-share).
    #[serde(default)]
    pub last_refresh: Option<String>,
    /// Last operation description (A-share).
    #[serde(default)]
    pub last_operation: Option<String>,
    /// Panic state (raw JSON for A-share, bool for TW).
    #[serde(default)]
    pub panic: Option<Value>,
    /// Whether audit writing is available (A-share).
    #[serde(default)]
    pub audit_writable: Option<bool>,
    /// Server version.
    #[serde(default)]
    pub version: Option<String>,
    /// Health response timestamp.
    #[serde(default)]
    pub timestamp_ms: Option<i64>,
    /// Whether the TW adapter is ready.
    #[serde(default)]
    pub adapter_ready: Option<bool>,
    /// Whether the TW login session is active.
    #[serde(default)]
    pub login_status: Option<bool>,
    /// TW event queue size.
    #[serde(default)]
    pub event_queue_size: Option<u64>,
    /// Whether TW auditing is enabled.
    #[serde(default)]
    pub audit_enabled: Option<bool>,
    /// TW audit file path.
    #[serde(default)]
    pub audit_file: Option<String>,
    /// TW environment name.
    #[serde(default)]
    pub environment: Option<String>,
    /// TW circuit breaker open flag.
    #[serde(default)]
    pub circuit_breaker_open: Option<bool>,
    /// TW circuit breaker details.
    #[serde(default)]
    pub circuit_breaker: Option<Value>,
    /// TW last failure details.
    #[serde(default)]
    pub last_failure: Option<Value>,
    /// TW last recovery details.
    #[serde(default)]
    pub last_recovery: Option<Value>,
    /// Unknown response fields.
    #[serde(default, flatten)]
    pub extra: Value,
}

/// Unified WebSocket event.
///
/// Known common events are kept as dedicated variants while server-specific
/// events and unknown events preserve the original `type`, `data`, and
/// `timestamp_ms` so callers never lose information.
#[derive(Debug, Clone, PartialEq)]
pub enum BrokerEvent {
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
    /// `health.changed`
    HealthChanged {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// `heartbeat`
    Heartbeat {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// A-share `account.balance_changed`
    AccountBalanceChanged {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// A-share `query.cache_hit`
    QueryCacheHit {
        /// Raw event payload.
        data: Value,
        /// `true` when served from cache.
        from_cache: bool,
        /// Cache snapshot time.
        cached_at: Option<String>,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// A-share `replace.updated`
    ReplaceUpdated {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// A-share `order.no_mapping`
    OrderNoMapping {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// A-share `order.manual_review`
    OrderManualReview {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// A-share `risk.panic`
    RiskPanic {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// TW `welcome`
    Welcome {
        /// Optional welcome message.
        message: Option<String>,
    },
    /// TW `Login`
    Login {
        /// Raw login payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// TW real report
    RealReport {
        /// Raw report payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// TW merged real report
    RealReportMerge {
        /// Raw report payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// TW `quote.updated`
    QuoteUpdated {
        /// Raw quote payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// TW subscribe acknowledgement events.
    Subscribed {
        /// Raw subscribe payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// Unknown / server-specific event.
    Unknown {
        /// Original `type` string.
        type_name: String,
        /// Raw event data (or full object when no `data` field exists).
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn tw_order_request_new_remains_compatible() {
        let req = OrderRequest::new("C1", "S98875005091", "2330", "B", 500.0, 10, "ROD", "LIMIT");
        assert_eq!(req.stk_code, "2330");
        assert_eq!(req.symbol.as_deref(), Some("2330"));
        assert_eq!(req.side.as_deref(), Some("B"));
        assert_eq!(req.price, Some(500.0));
        assert_eq!(req.quantity, Some(10));
        assert_eq!(req.action, OrderAction::New);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["action"], "new");
        assert_eq!(json["stk_code"], "2330");
        assert!(json.get("dry_run").is_none());
    }

    #[test]
    fn a_order_request_fields_are_kept_in_unified_type() {
        let req = OrderRequest::a_new("C2", "512100", "buy", 3.305, 100, true);
        assert_eq!(req.symbol.as_deref(), Some("512100"));
        assert_eq!(req.dry_run, Some(true));
        assert!(req.account.is_empty());
        assert_eq!(req.time_in_force, None);
    }

    #[test]
    fn position_superset_holds_a_and_tw_fields() {
        let pos = Position {
            account: Some("S1".to_owned()),
            symbol: Some("2330".to_owned()),
            stk_code: Some("2330".to_owned()),
            name: Some("台積電".to_owned()),
            stock_name: Some("台積電".to_owned()),
            quantity: Some(100.0),
            today_qty: Some(10.0),
            yesterday_qty: Some(90.0),
            ..Default::default()
        };
        assert_eq!(pos.stk_code.as_deref(), Some("2330"));
        assert_eq!(pos.symbol.as_deref(), Some("2330"));
        assert_eq!(pos.today_qty, Some(10.0));
        assert_eq!(pos.yesterday_qty, Some(90.0));
    }

    #[test]
    fn account_superset_holds_a_and_tw_fields() {
        let account = Account {
            account: Some("S1".to_owned()),
            total_asset: Some(1000.0),
            total_balance: Some(900.0),
            available_balance: Some(800.0),
            withdrawable: Some(700.0),
            from_cache: true,
            cached_at: Some("now".to_owned()),
            ..Default::default()
        };
        assert_eq!(account.total_asset, Some(1000.0));
        assert_eq!(account.total_balance, Some(900.0));
        assert!(account.from_cache);
    }

    #[test]
    fn broker_event_unknown_preserves_raw_fields() {
        let event = BrokerEvent::Unknown {
            type_name: "future.event".to_owned(),
            data: json!({"x": 1}),
            timestamp_ms: Some(42),
        };
        match event {
            BrokerEvent::Unknown {
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
}
