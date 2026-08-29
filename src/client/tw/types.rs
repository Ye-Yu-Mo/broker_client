//! Strongly-typed models for the TW server API.
//!
//! The TW API contains many nested, undocumented fields. Types in this module
//! use `#[serde(default)]` liberally and keep unknown fields in `extra` so a
//! server addition never breaks decoding.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A request body for `POST /api/v1/session/login`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LoginRequest {
    /// Optional account; omitted means server-default account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Optional account password.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Optional PFX certificate path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pfx_path: Option<String>,
    /// Optional PFX certificate password.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pfx_pass: Option<String>,
}

impl LoginRequest {
    /// Creates an empty login request (server defaults are used).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a login request with all optional fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account: impl Into<String>,
        password: impl Into<String>,
        pfx_path: impl Into<String>,
        pfx_pass: impl Into<String>,
    ) -> Self {
        Self {
            account: Some(account.into()),
            password: Some(password.into()),
            pfx_path: Some(pfx_path.into()),
            pfx_pass: Some(pfx_pass.into()),
        }
    }

    /// Sets the account.
    pub fn account(mut self, account: impl Into<String>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Sets the password.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Sets the PFX certificate path.
    pub fn pfx_path(mut self, pfx_path: impl Into<String>) -> Self {
        self.pfx_path = Some(pfx_path.into());
        self
    }

    /// Sets the PFX certificate password.
    pub fn pfx_pass(mut self, pfx_pass: impl Into<String>) -> Self {
        self.pfx_pass = Some(pfx_pass.into());
        self
    }
}

/// Health check response from `GET /health`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Health {
    /// Overall status.
    #[serde(default)]
    pub status: String,
    /// Whether the broker adapter is ready.
    #[serde(default)]
    pub adapter_ready: bool,
    /// Whether the login session is active.
    #[serde(default)]
    pub login_status: bool,
    /// Event queue size.
    #[serde(default)]
    pub event_queue_size: u64,
    /// Whether auditing is enabled.
    #[serde(default)]
    pub audit_enabled: bool,
    /// Audit file path, if any.
    #[serde(default)]
    pub audit_file: Option<String>,
    /// Server version.
    #[serde(default)]
    pub version: String,
    /// Environment name.
    #[serde(default)]
    pub environment: String,
    /// Panic mode flag.
    #[serde(default)]
    pub panic: bool,
    /// Circuit breaker open flag.
    #[serde(default)]
    pub circuit_breaker_open: bool,
    /// Circuit breaker details.
    #[serde(default)]
    pub circuit_breaker: Option<Value>,
    /// Last failure details.
    #[serde(default)]
    pub last_failure: Option<Value>,
    /// Last recovery details.
    #[serde(default)]
    pub last_recovery: Option<Value>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// One account inside the login response.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LoginInfo {
    /// Broker account.
    #[serde(default)]
    pub account: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Investor ID.
    #[serde(default)]
    pub investor_id: String,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// `login.login_list` wrapper.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LoginList {
    /// List of login accounts.
    #[serde(default)]
    pub login_list: Vec<LoginInfo>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Data returned by `POST /api/v1/session/login`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LoginResponse {
    /// Nested login information.
    #[serde(default)]
    pub login: Option<LoginList>,
    /// Active account.
    #[serde(default)]
    pub account: Option<String>,
    /// Display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Investor ID.
    #[serde(default)]
    pub investor_id: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Session status returned by `GET /api/v1/session/status`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SessionStatus {
    /// Whether the session is currently logged in.
    #[serde(default)]
    pub logged_in: bool,
    /// Optional active account.
    #[serde(default)]
    pub account: Option<String>,
    /// Optional display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional investor ID.
    #[serde(default)]
    pub investor_id: Option<String>,
    /// Optional raw status string.
    #[serde(default)]
    pub status: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A stock position.
///
/// This is an alias of the unified [`crate::types::Position`] super-set.
pub use crate::types::Position;

/// Account bank balance.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Balance {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// Total bank balance.
    #[serde(default)]
    pub total_balance: Option<f64>,
    /// Available balance.
    #[serde(default)]
    pub available_balance: Option<f64>,
    /// Frozen/reserved balance.
    #[serde(default)]
    pub frozen: Option<f64>,
    /// Withdrawable balance.
    #[serde(default)]
    pub withdrawable: Option<f64>,
    /// Last update time.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Settlement amount.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Settlement {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Settlement/trade date.
    #[serde(default)]
    pub trade_date: Option<String>,
    /// Currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// Settlement amount.
    #[serde(default)]
    pub amount: Option<f64>,
    /// Whether this is a receivable/payable amount.
    #[serde(default)]
    pub receive_pay: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Unrealized P&L.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PnlUnrealized {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Quantity.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Cost price.
    #[serde(default)]
    pub cost_price: Option<f64>,
    /// Current price.
    #[serde(default)]
    pub current_price: Option<f64>,
    /// Unrealized P&L amount.
    #[serde(default)]
    pub pnl: Option<f64>,
    /// Unrealized P&L ratio.
    #[serde(default)]
    pub pnl_ratio: Option<f64>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Realized P&L.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PnlRealized {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Quantity realized.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Realized P&L amount.
    #[serde(default)]
    pub realized_pnl: Option<f64>,
    /// Trade date.
    #[serde(default)]
    pub trade_date: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Reversal P&L.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PnlReversal {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Reversal gain/loss.
    #[serde(default)]
    pub re_gain_loss: Option<Value>,
    /// Reversal amount.
    #[serde(default)]
    pub amount: Option<f64>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A real-time report.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RealReport {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Broker order number.
    #[serde(default)]
    pub order_no: Option<String>,
    /// Trade date.
    #[serde(default)]
    pub trade_date: Option<String>,
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Side (`B`/`S`).
    #[serde(default)]
    pub side: Option<String>,
    /// Trade price.
    #[serde(default)]
    pub price: Option<f64>,
    /// Trade quantity.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A merged real-time report.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RealReportMerge {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Broker order number.
    #[serde(default)]
    pub order_no: Option<String>,
    /// Trade date.
    #[serde(default)]
    pub trade_date: Option<String>,
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Side (`B`/`S`).
    #[serde(default)]
    pub side: Option<String>,
    /// Trade price.
    #[serde(default)]
    pub price: Option<f64>,
    /// Trade quantity.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A combined order/trade report.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OrderTradeReport {
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Client order ID.
    #[serde(default)]
    pub client_order_id: Option<String>,
    /// Broker order number.
    #[serde(default)]
    pub order_no: Option<String>,
    /// Trade date.
    #[serde(default)]
    pub trade_date: Option<String>,
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Side (`B`/`S`).
    #[serde(default)]
    pub side: Option<String>,
    /// Price.
    #[serde(default)]
    pub price: Option<f64>,
    /// Quantity.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Order status.
    #[serde(default)]
    pub status: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Supported quote subscription type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteType {
    /// `watchlist`
    Watchlist,
    /// `watchlist_all`
    WatchlistAll,
    /// `five_tick`
    FiveTick,
    /// `stock_tick`
    StockTick,
    /// `market_info`
    MarketInfo,
    /// `stock_info`
    StockInfo,
}

/// Source filter for `GET /api/v1/quotes/subscribed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscribedSource {
    /// Local SQLite subscription list.
    Local,
    /// Broker-side actual subscription list.
    Broker,
    /// Both local and broker lists.
    Both,
}

/// Quote subscription request/list item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuoteSubscription {
    /// Subscription type.
    #[serde(rename = "type")]
    pub r#type: QuoteType,
    /// Stock symbols.
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Optional account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Optional market type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_type: Option<String>,
    /// Optional index flag (mainly used by `watchlist`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_flag: Option<i64>,
}

impl QuoteSubscription {
    /// Creates a basic subscription.
    pub fn new(r#type: QuoteType, symbols: Vec<String>) -> Self {
        Self {
            r#type,
            symbols,
            account: None,
            market_type: None,
            index_flag: None,
        }
    }

    /// Sets the account.
    pub fn account(mut self, account: impl Into<String>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Sets the market type.
    pub fn market_type(mut self, market_type: impl Into<String>) -> Self {
        self.market_type = Some(market_type.into());
        self
    }

    /// Sets the index flag.
    pub fn index_flag(mut self, index_flag: i64) -> Self {
        self.index_flag = Some(index_flag);
        self
    }
}

/// A quote snapshot.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct QuoteSnapshot {
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Market type.
    #[serde(default)]
    pub market_type: Option<String>,
    /// Stock name.
    #[serde(default)]
    pub name: Option<String>,
    /// Open price.
    #[serde(default)]
    pub open: Option<f64>,
    /// High price.
    #[serde(default)]
    pub high: Option<f64>,
    /// Low price.
    #[serde(default)]
    pub low: Option<f64>,
    /// Close/previous close price.
    #[serde(default)]
    pub close: Option<f64>,
    /// Last traded price.
    #[serde(default)]
    pub last_price: Option<f64>,
    /// Best bid price.
    #[serde(default)]
    pub bid: Option<f64>,
    /// Best ask price.
    #[serde(default)]
    pub ask: Option<f64>,
    /// Total volume.
    #[serde(default)]
    pub volume: Option<f64>,
    /// Snapshot timestamp.
    #[serde(default)]
    pub timestamp: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A single tick.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Tick {
    /// Tick time.
    #[serde(default)]
    pub time: Option<String>,
    /// Tick price.
    #[serde(default)]
    pub price: Option<f64>,
    /// Tick volume.
    #[serde(default)]
    pub volume: Option<f64>,
    /// Tick amount.
    #[serde(default)]
    pub amount: Option<f64>,
    /// Tick side if provided.
    #[serde(default)]
    pub side: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// One price/volume row in a classify-price response.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ClassifyPrice {
    /// Price level.
    #[serde(default)]
    pub price: Option<f64>,
    /// Volume at this price.
    #[serde(default)]
    pub volume: Option<f64>,
    /// Amount at this price.
    #[serde(default)]
    pub amount: Option<f64>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// A K-line bar.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Kline {
    /// Bar date/time.
    #[serde(default)]
    pub date: Option<String>,
    /// Open price.
    #[serde(default)]
    pub open: Option<f64>,
    /// High price.
    #[serde(default)]
    pub high: Option<f64>,
    /// Low price.
    #[serde(default)]
    pub low: Option<f64>,
    /// Close price.
    #[serde(default)]
    pub close: Option<f64>,
    /// Volume.
    #[serde(default)]
    pub volume: Option<f64>,
    /// Amount.
    #[serde(default)]
    pub amount: Option<f64>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Stock information.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StockInfo {
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Market type.
    #[serde(default)]
    pub market_type: Option<String>,
    /// Stock name.
    #[serde(default)]
    pub name: Option<String>,
    /// Industry.
    #[serde(default)]
    pub industry: Option<String>,
    /// Sector.
    #[serde(default)]
    pub sector: Option<String>,
    /// Listing date.
    #[serde(default)]
    pub listing_date: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Order action for `POST /api/v1/orders/stock`.
///
/// Re-exported from the unified type model.
pub use crate::types::OrderAction;

/// A type-safe TW order request.
///
/// This is an alias of the unified [`crate::types::OrderRequest`] super-set;
/// the TW constructors remain available on that type.
pub use crate::types::OrderRequest;

/// Order status payload returned by order endpoints and pushed by WebSocket.
///
/// This is an alias of the unified [`crate::types::OrderStatus`] super-set.
pub use crate::types::OrderStatus;

/// A full order record returned by order query endpoints.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OrderRecord {
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
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Side (`B`/`S`).
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
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// An unresolved recovery item.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RecoveryItem {
    /// Client order ID.
    #[serde(default)]
    pub client_order_id: Option<String>,
    /// Source table: `orders` or `stock_orders`.
    #[serde(default)]
    pub source: Option<String>,
    /// Broker order number.
    #[serde(default)]
    pub order_no: Option<String>,
    /// Trade date.
    #[serde(default)]
    pub trade_date: Option<String>,
    /// Order status.
    #[serde(default)]
    pub status: Option<String>,
    /// Account number.
    #[serde(default)]
    pub account: Option<String>,
    /// Stock code.
    #[serde(default)]
    pub stk_code: Option<String>,
    /// Created time.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Unknown response fields.
    #[serde(default, flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// Request body for `POST /api/v1/recovery/{client_order_id}/resolve`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RecoveryResolveRequest {
    /// Final status to set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Broker order number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_no: Option<String>,
    /// Trade date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_date: Option<String>,
    /// Source table: `orders` or `stock_orders`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Human note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl RecoveryResolveRequest {
    /// Creates a new resolution request.
    pub fn new(status: impl Into<String>) -> Self {
        Self {
            status: Some(status.into()),
            ..Self::default()
        }
    }

    /// Sets the broker order number.
    pub fn order_no(mut self, order_no: impl Into<String>) -> Self {
        self.order_no = Some(order_no.into());
        self
    }

    /// Sets the trade date.
    pub fn trade_date(mut self, trade_date: impl Into<String>) -> Self {
        self.trade_date = Some(trade_date.into());
        self
    }

    /// Sets the source table.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets a human note.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// WebSocket event pushed by the TW server.
///
/// The event is dispatched by its `type` field. Unknown event types are kept
/// in [`TwEvent::Unknown`] so a future server addition never breaks parsing.
#[derive(Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
#[allow(clippy::large_enum_variant)]
pub enum TwEvent {
    /// `welcome`
    Welcome {
        /// Optional welcome message.
        message: Option<String>,
    },
    /// `Login`
    Login(LoginResponse),
    /// `RR_RealReport`
    RR_RealReport(Value),
    /// `RR_RealReportMerge`
    RR_RealReportMerge(Value),
    /// `real_report`
    RealReport(RealReport),
    /// `real_report_merge`
    RealReportMerge(RealReportMerge),
    /// `order.updated`
    OrderUpdated(OrderStatus),
    /// `quote.updated`
    QuoteUpdated(Value),
    /// `heartbeat`
    Heartbeat(Value),
    /// `SubscribeWatchlist`
    SubscribeWatchlist(Value),
    /// `SubscribeWatchlistAll`
    SubscribeWatchlistAll(Value),
    /// `SubscribeFiveTickA`
    SubscribeFiveTickA(Value),
    /// `SubscribeStockTick`
    SubscribeStockTick(Value),
    /// `SubscribeMarketInformation`
    SubscribeMarketInformation(Value),
    /// `SubscribeStockInformation`
    SubscribeStockInformation(Value),
    /// Any other event type.
    Unknown {
        /// Original `type` string.
        type_name: String,
        /// Raw event data (or the full object when no `data` field exists).
        data: Value,
    },
}

impl<'de> serde::Deserialize<'de> for TwEvent {
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
        let data = value.get("data").cloned().unwrap_or(value.clone());

        Ok(match type_name.as_str() {
            "welcome" => TwEvent::Welcome {
                message: data
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
            "Login" => {
                TwEvent::Login(serde_json::from_value(data).map_err(serde::de::Error::custom)?)
            }
            "RR_RealReport" => TwEvent::RR_RealReport(data),
            "RR_RealReportMerge" => TwEvent::RR_RealReportMerge(data),
            "real_report" => {
                TwEvent::RealReport(serde_json::from_value(data).map_err(serde::de::Error::custom)?)
            }
            "real_report_merge" => TwEvent::RealReportMerge(
                serde_json::from_value(data).map_err(serde::de::Error::custom)?,
            ),
            "order.updated" => TwEvent::OrderUpdated(
                serde_json::from_value(data).map_err(serde::de::Error::custom)?,
            ),
            "quote.updated" => TwEvent::QuoteUpdated(data),
            "heartbeat" => TwEvent::Heartbeat(data),
            "SubscribeWatchlist" => TwEvent::SubscribeWatchlist(data),
            "SubscribeWatchlistAll" => TwEvent::SubscribeWatchlistAll(data),
            "SubscribeFiveTickA" => TwEvent::SubscribeFiveTickA(data),
            "SubscribeStockTick" => TwEvent::SubscribeStockTick(data),
            "SubscribeMarketInformation" => TwEvent::SubscribeMarketInformation(data),
            "SubscribeStockInformation" => TwEvent::SubscribeStockInformation(data),
            _ => TwEvent::Unknown { type_name, data },
        })
    }
}

impl From<TwEvent> for crate::types::BrokerEvent {
    fn from(event: TwEvent) -> Self {
        use crate::types::BrokerEvent;

        match event {
            TwEvent::Welcome { message } => BrokerEvent::Welcome { message },
            TwEvent::Login(login) => BrokerEvent::Login {
                data: serde_json::json!({
                    "account": login.account,
                    "name": login.name,
                    "investor_id": login.investor_id,
                    "login": login.login.map(|login_list| serde_json::json!({
                        "login_list": login_list.login_list.into_iter().map(|info| serde_json::json!({
                            "account": info.account,
                            "name": info.name,
                            "investor_id": info.investor_id,
                        })).collect::<Vec<_>>(),
                    })),
                }),
                timestamp_ms: None,
            },
            TwEvent::RR_RealReport(data) => BrokerEvent::RealReport {
                data,
                timestamp_ms: None,
            },
            TwEvent::RR_RealReportMerge(data) => BrokerEvent::RealReportMerge {
                data,
                timestamp_ms: None,
            },
            TwEvent::RealReport(report) => BrokerEvent::RealReport {
                data: serde_json::json!({
                    "account": report.account,
                    "order_no": report.order_no,
                    "trade_date": report.trade_date,
                    "stk_code": report.stk_code,
                    "side": report.side,
                    "price": report.price,
                    "quantity": report.quantity,
                }),
                timestamp_ms: None,
            },
            TwEvent::RealReportMerge(report) => BrokerEvent::RealReportMerge {
                data: serde_json::json!({
                    "account": report.account,
                    "order_no": report.order_no,
                    "trade_date": report.trade_date,
                    "stk_code": report.stk_code,
                    "side": report.side,
                    "price": report.price,
                    "quantity": report.quantity,
                }),
                timestamp_ms: None,
            },
            TwEvent::OrderUpdated(order) => BrokerEvent::OrderUpdated {
                data: serde_json::to_value(&order).unwrap_or(serde_json::Value::Null),
                timestamp_ms: None,
            },
            TwEvent::QuoteUpdated(data) => BrokerEvent::QuoteUpdated {
                data,
                timestamp_ms: None,
            },
            TwEvent::Heartbeat(data) => BrokerEvent::Heartbeat {
                data,
                timestamp_ms: None,
            },
            TwEvent::SubscribeWatchlist(data)
            | TwEvent::SubscribeWatchlistAll(data)
            | TwEvent::SubscribeFiveTickA(data)
            | TwEvent::SubscribeStockTick(data)
            | TwEvent::SubscribeMarketInformation(data)
            | TwEvent::SubscribeStockInformation(data) => BrokerEvent::Subscribed {
                data,
                timestamp_ms: None,
            },
            TwEvent::Unknown { type_name, data } => BrokerEvent::Unknown {
                type_name,
                data,
                timestamp_ms: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn login_response_parses_documented_example() {
        let value: LoginResponse = serde_json::from_value(json!({
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
        }))
        .unwrap();
        assert_eq!(value.account.as_deref(), Some("S98875005091"));
        assert_eq!(value.login.unwrap().login_list[0].name, "測試用戶");
    }

    #[test]
    fn position_parses_with_unknown_fields() {
        let pos: Position = serde_json::from_value(json!({
            "account": "S1",
            "stk_code": "2330",
            "quantity": 100,
            "future_field": {"x": 1}
        }))
        .unwrap();
        assert_eq!(pos.stk_code.as_deref(), Some("2330"));
        assert_eq!(pos.extra["future_field"]["x"], 1);
    }

    #[test]
    fn order_status_parses_websocket_payload() {
        let status: OrderStatus = serde_json::from_value(json!({
            "client_order_id": "C001",
            "status": "FILLED",
            "order_no": "H00001",
            "trade_date": "2026/08/28",
            "request": {},
            "data": {},
            "last_error": null
        }))
        .unwrap();
        assert_eq!(status.status.as_deref(), Some("FILLED"));
    }

    #[test]
    fn replace_requests_are_mutually_exclusive() {
        let price = OrderRequest::replace_price("C1", "A", "H1", "2330", "B", 510.0);
        let qty = OrderRequest::replace_quantity("C2", "A", "H1", "2330", "B", 20);
        assert_eq!(price.action, OrderAction::Replace);
        assert!(price.new_price.is_some());
        assert!(price.new_quantity.is_none());
        assert!(qty.new_price.is_none());
        assert!(qty.new_quantity.is_some());
    }

    #[test]
    fn quote_subscription_serializes_documented_body() {
        let sub = QuoteSubscription {
            r#type: QuoteType::FiveTick,
            symbols: vec!["2330".to_owned(), "2885".to_owned()],
            account: Some("S98875005091".to_owned()),
            market_type: Some("TWSE".to_owned()),
            index_flag: Some(7),
        };
        let json = serde_json::to_value(&sub).unwrap();
        assert_eq!(json["type"], "five_tick");
        assert_eq!(json["symbols"][0], "2330");
        assert_eq!(json["index_flag"], 7);
    }

    #[test]
    fn recovery_resolve_request_serializes_optional_fields_only() {
        let req = RecoveryResolveRequest::new("FILLED")
            .order_no("H00001")
            .trade_date("2026/08/28")
            .source("stock_orders")
            .note("人工確認");
        let json = serde_json::to_value(req).unwrap();
        assert_eq!(json["status"], "FILLED");
        assert_eq!(json["source"], "stock_orders");
        assert!(json.get("order_no").is_some());
    }

    #[test]
    fn tw_event_parses_known_types() {
        let event: TwEvent = serde_json::from_value(json!({
            "type": "order.updated",
            "data": {"client_order_id": "C001", "status": "FILLED"}
        }))
        .unwrap();
        assert!(matches!(event, TwEvent::OrderUpdated(_)));

        let event: TwEvent = serde_json::from_value(json!({
            "type": "welcome",
            "message": "connected"
        }))
        .unwrap();
        assert!(matches!(event, TwEvent::Welcome { message: Some(m) } if m == "connected"));

        let event: TwEvent = serde_json::from_value(json!({
            "type": "quote.updated",
            "data": {"stk_code": "2330"}
        }))
        .unwrap();
        assert!(matches!(event, TwEvent::QuoteUpdated(_)));
    }

    #[test]
    fn tw_event_unknown_type_is_preserved() {
        let event: TwEvent = serde_json::from_value(json!({
            "type": "future.event",
            "data": {"hello": 1}
        }))
        .unwrap();
        assert!(matches!(
            event,
            TwEvent::Unknown { type_name, data } if type_name == "future.event" && data["hello"] == 1
        ));
    }

    #[test]
    fn tw_event_converts_to_unified_broker_event() {
        let event: TwEvent = serde_json::from_value(json!({
            "type": "order.updated",
            "data": {"client_order_id": "C1", "status": "FILLED"}
        }))
        .unwrap();
        let unified: crate::types::BrokerEvent = event.into();
        assert!(matches!(
            unified,
            crate::types::BrokerEvent::OrderUpdated { .. }
        ));

        let unknown: TwEvent = serde_json::from_value(json!({
            "type": "future.event",
            "data": {"hello": 1}
        }))
        .unwrap();
        let unified: crate::types::BrokerEvent = unknown.into();
        match unified {
            crate::types::BrokerEvent::Unknown {
                type_name,
                data,
                timestamp_ms,
            } => {
                assert_eq!(type_name, "future.event");
                assert_eq!(data["hello"], 1);
                assert_eq!(timestamp_ms, None);
            }
            _ => panic!("expected unknown"),
        }
    }
}
