//! Broker client library for the TW and A-share stock servers.
//!
//! This crate provides a shared async HTTP foundation (`ClientConfig`,
//! `HttpClient`, `Error`) plus feature-gated server clients:
//!
//! - `TwClient` under the `client-tw` feature (default).
//! - `AClient` under the `client-a` feature (default).
//! - WebSocket dependencies/APIs under the `ws` feature.

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod http;
pub mod response;

#[cfg(feature = "client-tw")]
pub use client::TwClient;

#[cfg(feature = "client-tw")]
pub use client::tw::{
    Balance, ClassifyPrice, Health, Kline, LoginInfo, LoginList, LoginRequest, LoginResponse,
    OrderAction, OrderRecord, OrderRequest, OrderStatus, OrderTradeReport, PnlRealized,
    PnlReversal, PnlUnrealized, Position, QuoteSnapshot, QuoteSubscription, QuoteType, RealReport,
    RealReportMerge, RecoveryItem, RecoveryResolveRequest, SessionStatus, Settlement, StockInfo,
    SubscribedSource, Tick, TwEvent,
};

#[cfg(feature = "client-a")]
pub use client::AClient;

#[cfg(feature = "client-a")]
pub use client::a::{
    AEvent, AccountFunds, Cached, CancelRequest, Order, PanicRequest, Pnl, RefreshResponse,
    ReplaceRequest, Trade, Transaction,
};

#[cfg(feature = "client-a")]
pub use client::a::Health as AHealth;
#[cfg(feature = "client-a")]
pub use client::a::OrderRequest as AOrderRequest;
#[cfg(feature = "client-a")]
pub use client::a::Position as APosition;

pub use auth::AuthMethod;
pub use config::ClientConfig;
pub use error::{Error, Result};
