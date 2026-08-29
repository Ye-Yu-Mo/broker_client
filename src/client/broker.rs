//! Unified [`BrokerClient`] trait.
//!
//! Both the A-share and TW clients implement this trait so callers can write
//! one piece of code and use it with either server. The trait intentionally
//! uses only the unified types from [`crate::types`].

use async_trait::async_trait;

use crate::config::ClientConfig;
use crate::error::Result;
use crate::types::{Account, CancelOrderRequest, Health, OrderRequest, OrderStatus, Position};

#[cfg(feature = "ws")]
use std::pin::Pin;

#[cfg(feature = "ws")]
use futures_util::Stream;

#[cfg(feature = "ws")]
use crate::types::BrokerEvent;

/// Common broker operations shared by A-share and TW clients.
///
/// The trait is object-safe with the help of `async-trait`, so it can be used
/// as `Box<dyn BrokerClient>`.
#[async_trait]
pub trait BrokerClient: Send + Sync {
    /// Returns the client configuration.
    fn config(&self) -> &ClientConfig;

    /// Returns the server health as a unified value.
    async fn health(&self) -> Result<Health>;

    /// Returns the unified account summary.
    async fn account(&self) -> Result<Account>;

    /// Returns all positions as unified values.
    async fn positions(&self) -> Result<Vec<Position>>;

    /// Submits a new order using the unified request model.
    async fn submit_order(&self, request: &OrderRequest) -> Result<OrderStatus>;

    /// Cancels an order using the unified cancel request model.
    async fn cancel_order(&self, request: &CancelOrderRequest) -> Result<OrderStatus>;

    /// Gets one order by client order ID.
    async fn get_order(&self, client_order_id: &str) -> Result<OrderStatus>;

    /// Returns an auto-reconnecting stream of unified [`BrokerEvent`]s.
    #[cfg(feature = "ws")]
    async fn event_stream(&self) -> Result<Pin<Box<dyn Stream<Item = BrokerEvent> + Send>>>;
}
