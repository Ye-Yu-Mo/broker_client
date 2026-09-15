//! Unified type model shared by every broker implementation.
//!
//! The A-share and Taiwan clients historically maintained separate models. This
//! module defines the common super-set used by [`crate::client::BrokerClient`]:
//! A/TW specific fields are preserved as `Option<T>` so no server-specific
//! information is lost.
//!
//! # Design debt
//!
//! [`OrderRequest`] is currently the union of two different wire contracts:
//! `side` carries both `buy/sell` and `B/S`, an empty `account` means "not
//! applicable", and `symbol` / `dry_run` use serialization skips to pretend
//! fields do not exist on one server. Adding more optional fields to this union
//! makes the model worse, not more unified. The next expansion should replace
//! it with an explicit sum type such as
//! `enum UnifiedOrder { A(AOrderRequest), Tw(TwStockOrderRequest) }`, or with
//! market-specific associated request types on a generic trait. Do not add a
//! third market by extending this struct with another pile of `Option`s.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unified order action.
///
/// This is the same enum historically exposed by the TW client; it is kept here
/// so the unified request type can represent new/cancel/replace uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderAction {
    /// New order.
    #[default]
    New,
    /// Cancel order.
    Cancel,
    /// Replace order (price or quantity).
    Replace,
}

/// TW domestic-stock order category (`ap_code`).
///
/// Requests serialize the readable semantic names accepted by the server. The
/// custom deserializer also accepts the legacy SDK integers `0/2/4/7`, so old
/// persisted responses remain readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApCode {
    /// Regular board-lot trading (`0`).
    Regular,
    /// Odd-lot trading (`2`).
    OddLot,
    /// Intraday odd-lot trading (`4`).
    IntradayOddLot,
    /// After-hours trading (`7`).
    AfterHours,
}

impl ApCode {
    fn from_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "REGULAR" => Some(Self::Regular),
            "ODD_LOT" => Some(Self::OddLot),
            "INTRADAY_ODD_LOT" => Some(Self::IntradayOddLot),
            "AFTER_HOURS" => Some(Self::AfterHours),
            _ => None,
        }
    }

    fn invalid(value: impl std::fmt::Display) -> crate::Error {
        crate::Error::InvalidRequest(format!(
            "invalid ap_code {value}; expected REGULAR/0, ODD_LOT/2, INTRADAY_ODD_LOT/4, or AFTER_HOURS/7"
        ))
    }
}

impl TryFrom<i32> for ApCode {
    type Error = crate::Error;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Regular),
            2 => Ok(Self::OddLot),
            4 => Ok(Self::IntradayOddLot),
            7 => Ok(Self::AfterHours),
            _ => Err(Self::invalid(value)),
        }
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<u8> for ApCode {
    /// Converts a legacy SDK value.
    ///
    /// # Panics
    ///
    /// Panics when `value` is not one of `0/2/4/7`. Use [`ApCode::try_from`]
    /// for untrusted input; silently mapping an unknown trading category to
    /// `Regular` could place the wrong kind of order.
    fn from(value: u8) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_else(|_| panic!("invalid ap_code {value}"))
    }
}

impl<'de> Deserialize<'de> for ApCode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ApCodeVisitor;

        impl serde::de::Visitor<'_> for ApCodeVisitor {
            type Value = ApCode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("REGULAR/0, ODD_LOT/2, INTRADAY_ODD_LOT/4, or AFTER_HOURS/7")
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let value = i32::try_from(value).map_err(E::custom)?;
                ApCode::try_from(value).map_err(E::custom)
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let value = i32::try_from(value).map_err(E::custom)?;
                ApCode::try_from(value).map_err(E::custom)
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Some(code) = ApCode::from_name(value) {
                    return Ok(code);
                }
                value
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| E::custom(ApCode::invalid(value)))
                    .and_then(|value| ApCode::try_from(value).map_err(E::custom))
            }
        }

        deserializer.deserialize_any(ApCodeVisitor)
    }
}

/// Unified order request.
///
/// The type is a superset of both the A-share and TW order request shapes.
/// Common fields are `client_order_id`, `symbol`, `side`, `price`, and
/// `quantity`; server-specific fields are optional.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
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
    /// TW domestic-stock order category.
    ///
    /// `None` omits the field so the server applies its `REGULAR` default and
    /// existing request bodies stay byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ap_code: Option<ApCode>,
    /// Mock-mode flag.
    ///
    /// A mock order is settled against the server-side simulated account
    /// instead of reaching the broker. `None` omits the field entirely so the
    /// body stays byte-identical to 0.3.0; both servers default it to `false`,
    /// so `None` and `Some(false)` are equivalent on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mock: Option<bool>,
}

impl OrderRequest {
    /// Sets the TW domestic-stock order category.
    pub fn with_ap_code(mut self, ap_code: ApCode) -> Self {
        self.ap_code = Some(ap_code);
        self
    }

    /// Sets the mock-mode flag.
    ///
    /// This is the only way to opt into mock trading: every constructor leaves
    /// the flag unset, which keeps existing request bodies unchanged.
    pub fn with_mock(mut self, mock: bool) -> Self {
        self.mock = mock.then_some(true);
        self
    }

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
            ap_code: None,
            mock: None,
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
            ap_code: None,
            mock: None,
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
            ap_code: None,
            mock: None,
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
            ap_code: None,
            mock: None,
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
            ap_code: None,
            mock: None,
        }
    }
}

/// One initial position in the unified mock-account model.
///
/// Implementations map `code` to A's `symbol` or TW's `stk_code`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MockPositionInit {
    /// Market-specific stock code.
    pub code: String,
    /// Initial total quantity.
    pub quantity: f64,
    /// Initial tradable quantity (A-share only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_quantity: Option<f64>,
    /// Initial average cost (`average_cost` on A, `avg_price` on TW).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_cost: Option<f64>,
}

/// Unified request for initializing a server-side mock account.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MockAccountInitRequest {
    /// Mock-account identifier, required by TW and ignored by A-share.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Initial simulated cash.
    pub cash: f64,
    /// Initial simulated positions.
    #[serde(default)]
    pub positions: Vec<MockPositionInit>,
    /// Whether an existing account may be overwritten (A-share only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<bool>,
}

/// Unified server-side mock account.
///
/// A-share has one unnamed account with millisecond timestamps. TW has named,
/// activatable accounts with ISO-8601 timestamps. The two timestamp pairs and
/// `active` therefore remain optional instead of inventing a lossy conversion.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MockAccount {
    /// Mock-account identifier (TW only).
    #[serde(default)]
    pub account: Option<String>,
    /// Simulated available cash.
    #[serde(default)]
    pub cash: f64,
    /// Normalized simulated positions.
    #[serde(default)]
    pub positions: Vec<MockPositionInit>,
    /// Whether the account accepts orders (TW only).
    #[serde(default)]
    pub active: Option<bool>,
    /// Creation time as ISO 8601 (TW only).
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last update time as ISO 8601 (TW only).
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Creation time as epoch milliseconds (A-share only).
    #[serde(default)]
    pub created_at_ms: Option<u64>,
    /// Last update time as epoch milliseconds (A-share only).
    #[serde(default)]
    pub updated_at_ms: Option<u64>,
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
    #[serde(default, alias = "filled_qty")]
    pub filled_quantity: Option<f64>,
    /// Execution/fill price when the server reports one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_price: Option<f64>,
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
    /// Whether this order used the server-side mock execution path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mock: Option<bool>,
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
    /// A-share `mock.account_changed`.
    MockAccountChanged {
        /// Raw event payload.
        data: Value,
        /// Server timestamp in milliseconds.
        timestamp_ms: Option<i64>,
    },
    /// A-share `ws.lagged`; some broadcast events were dropped before delivery.
    WsLagged {
        /// Raw event payload, including `skipped`.
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

    #[test]
    fn tw_order_request_without_mock_keeps_the_030_wire_bytes() {
        // Regression guard: both `mock` and `ap_code` must be opt-in. Omitting
        // them has to produce the exact 0.3.0 body, byte for byte.
        let request = OrderRequest::new("C1", "S1", "2330", "B", 500.0, 10, "ROD", "LIMIT");
        assert_eq!(request.mock, None);
        assert_eq!(request.ap_code, None);
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"client_order_id":"C1","action":"new","account":"S1","stk_code":"2330","side":"B","price":500.0,"quantity":10,"time_in_force":"ROD","price_flag":"LIMIT"}"#
        );
    }

    #[test]
    fn with_mock_serializes_the_flag_after_the_existing_fields() {
        let request =
            OrderRequest::new("C1", "S1", "2330", "B", 500.0, 10, "ROD", "LIMIT").with_mock(true);
        assert_eq!(request.mock, Some(true));
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"client_order_id":"C1","action":"new","account":"S1","stk_code":"2330","side":"B","price":500.0,"quantity":10,"time_in_force":"ROD","price_flag":"LIMIT","mock":true}"#
        );
    }

    #[test]
    fn with_mock_false_is_omitted_for_wire_compatibility() {
        // Both servers default `mock` to `false`; explicit false therefore
        // must behave like the unset builder state and add no wire field.
        let explicit =
            OrderRequest::new("C1", "S1", "2330", "B", 500.0, 10, "ROD", "LIMIT").with_mock(false);
        assert_eq!(explicit.mock, None);
        assert!(!serde_json::to_string(&explicit).unwrap().contains("mock"));

        for constructor in [
            OrderRequest::new("C1", "S1", "2330", "B", 500.0, 10, "ROD", "LIMIT"),
            OrderRequest::a_new("C2", "512100", "buy", 3.305, 100, false),
        ] {
            assert_eq!(constructor.mock, None);
            assert!(
                !serde_json::to_string(&constructor)
                    .unwrap()
                    .contains("mock")
            );
        }
    }

    #[test]
    fn a_new_carries_mock_through_the_unified_request() {
        let request = OrderRequest::a_new("C1", "512100", "buy", 3.305, 100, true).with_mock(true);
        assert_eq!(request.mock, Some(true));
        assert_eq!(request.symbol.as_deref(), Some("512100"));
    }

    #[test]
    fn ap_code_serializes_to_semantic_strings() {
        let cases = [
            (ApCode::Regular, "REGULAR"),
            (ApCode::OddLot, "ODD_LOT"),
            (ApCode::IntradayOddLot, "INTRADAY_ODD_LOT"),
            (ApCode::AfterHours, "AFTER_HOURS"),
        ];
        for (code, expected) in cases {
            assert_eq!(
                serde_json::to_string(&code).unwrap(),
                format!(r#""{expected}""#)
            );
        }
    }

    #[test]
    fn ap_code_deserializes_semantic_strings_and_legacy_numbers() {
        let cases = [
            (json!("REGULAR"), ApCode::Regular),
            (json!("ODD_LOT"), ApCode::OddLot),
            (json!("INTRADAY_ODD_LOT"), ApCode::IntradayOddLot),
            (json!("AFTER_HOURS"), ApCode::AfterHours),
            (json!(0), ApCode::Regular),
            (json!(2), ApCode::OddLot),
            (json!(4), ApCode::IntradayOddLot),
            (json!(7), ApCode::AfterHours),
        ];
        for (value, expected) in cases {
            assert_eq!(serde_json::from_value::<ApCode>(value).unwrap(), expected);
        }
    }

    #[test]
    fn ap_code_rejects_unknown_names_and_numbers() {
        for value in [
            json!("AUCTION"),
            json!(1),
            json!(-1),
            json!(256),
            json!(true),
        ] {
            assert!(
                serde_json::from_value::<ApCode>(value.clone()).is_err(),
                "{value} must be rejected rather than silently becoming REGULAR"
            );
        }
        for value in [-1, 1, 3, 5, 6, 8, 256] {
            assert!(ApCode::try_from(value).is_err());
        }
    }

    #[test]
    fn ap_code_converts_from_legacy_numeric_values() {
        assert_eq!(ApCode::from(0_u8), ApCode::Regular);
        assert_eq!(ApCode::from(2_u8), ApCode::OddLot);
        assert_eq!(ApCode::from(4_u8), ApCode::IntradayOddLot);
        assert_eq!(ApCode::from(7_u8), ApCode::AfterHours);

        assert_eq!(ApCode::try_from(0_i32).unwrap(), ApCode::Regular);
        assert_eq!(ApCode::try_from(2_i32).unwrap(), ApCode::OddLot);
        assert_eq!(ApCode::try_from(4_i32).unwrap(), ApCode::IntradayOddLot);
        assert_eq!(ApCode::try_from(7_i32).unwrap(), ApCode::AfterHours);
    }

    #[test]
    fn tw_order_request_with_ap_code_adds_only_that_field() {
        let request = OrderRequest::new("C1", "S1", "2330", "B", 500.0, 10, "ROD", "LIMIT")
            .with_ap_code(ApCode::OddLot);
        assert_eq!(request.ap_code, Some(ApCode::OddLot));
        assert_eq!(request.mock, None);
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"client_order_id":"C1","action":"new","account":"S1","stk_code":"2330","side":"B","price":500.0,"quantity":10,"time_in_force":"ROD","price_flag":"LIMIT","ap_code":"ODD_LOT"}"#
        );
    }

    #[test]
    fn unified_mock_init_model_holds_both_server_shapes() {
        let request = MockAccountInitRequest {
            account: Some("MOCK-TEST".to_owned()),
            cash: 100_000.0,
            positions: vec![MockPositionInit {
                code: "2330".to_owned(),
                quantity: 10.0,
                available_quantity: Some(8.0),
                average_cost: Some(500.0),
            }],
            reset: Some(false),
        };
        assert_eq!(request.account.as_deref(), Some("MOCK-TEST"));
        assert_eq!(request.positions[0].code, "2330");
        assert_eq!(request.positions[0].available_quantity, Some(8.0));
        assert_eq!(request.reset, Some(false));
    }

    #[test]
    fn unified_mock_account_represents_a_and_tw_metadata() {
        let a = MockAccount {
            cash: 100_000.0,
            positions: vec![],
            created_at_ms: Some(1),
            updated_at_ms: Some(2),
            ..Default::default()
        };
        assert_eq!(a.account, None);
        assert_eq!(a.active, None);
        assert_eq!(a.created_at_ms, Some(1));

        let tw = MockAccount {
            account: Some("MOCK-TEST".to_owned()),
            cash: 100_000.0,
            active: Some(true),
            created_at: Some("2026-09-14T00:00:00+00:00".to_owned()),
            updated_at: Some("2026-09-14T00:00:01+00:00".to_owned()),
            ..Default::default()
        };
        assert_eq!(tw.account.as_deref(), Some("MOCK-TEST"));
        assert_eq!(tw.active, Some(true));
        assert_eq!(tw.created_at_ms, None);
    }

    #[test]
    fn order_status_parses_mock_as_a_typed_field() {
        let status: OrderStatus = serde_json::from_value(json!({
            "client_order_id": "M1",
            "status": "FILLED",
            "mock": true,
            "future_field": 1
        }))
        .unwrap();
        assert_eq!(status.mock, Some(true));
        assert_eq!(status.extra["future_field"], 1);
        assert!(status.extra.get("mock").is_none());

        let old: OrderStatus = serde_json::from_value(json!({"status": "SUBMITTED"})).unwrap();
        assert_eq!(old.mock, None);
    }

    #[test]
    fn order_status_new_optional_fields_do_not_change_legacy_json_when_absent() {
        let status = OrderStatus::default();
        let json = serde_json::to_value(status).unwrap();
        assert!(json.get("mock").is_none());
        assert!(json.get("fill_price").is_none());
    }

    #[test]
    fn order_status_serializes_new_fields_when_present() {
        let status = OrderStatus {
            mock: Some(false),
            fill_price: Some(101.0),
            ..Default::default()
        };
        let json = serde_json::to_value(status).unwrap();
        assert_eq!(json["mock"], false);
        assert_eq!(json["fill_price"], 101.0);
    }
}
