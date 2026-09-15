//! Unified [`BrokerClient`] trait.
//!
//! Both the A-share and TW clients implement this trait so callers can write
//! one piece of code and use it with either server. The trait intentionally
//! uses only the unified types from [`crate::types`].

use async_trait::async_trait;

use crate::config::ClientConfig;
use crate::error::Result;
use crate::types::{
    Account, CancelOrderRequest, Health, MockAccount, MockAccountInitRequest, OrderRequest,
    OrderStatus, Position,
};

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

    /// Returns the mock account selected by this client.
    ///
    /// Implementations without mock-account support inherit an error instead of
    /// being broken by this additive trait method.
    async fn mock_account(&self) -> Result<MockAccount> {
        Err(crate::Error::InvalidRequest(
            "mock account is not supported".to_owned(),
        ))
    }

    /// Initializes the server-side mock account.
    ///
    /// Implementations without mock-account support inherit an error instead of
    /// being broken by this additive trait method.
    async fn init_mock_account(&self, _request: &MockAccountInitRequest) -> Result<MockAccount> {
        Err(crate::Error::InvalidRequest(
            "mock account is not supported".to_owned(),
        ))
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    struct ThirdPartyClient {
        config: ClientConfig,
    }

    fn unsupported() -> crate::Error {
        crate::Error::InvalidRequest("not implemented by test client".to_owned())
    }

    #[async_trait]
    impl BrokerClient for ThirdPartyClient {
        fn config(&self) -> &ClientConfig {
            &self.config
        }

        async fn health(&self) -> Result<Health> {
            Err(unsupported())
        }

        async fn account(&self) -> Result<Account> {
            Err(unsupported())
        }

        async fn positions(&self) -> Result<Vec<Position>> {
            Err(unsupported())
        }

        async fn submit_order(&self, _request: &OrderRequest) -> Result<OrderStatus> {
            Err(unsupported())
        }

        async fn cancel_order(&self, _request: &CancelOrderRequest) -> Result<OrderStatus> {
            Err(unsupported())
        }

        async fn get_order(&self, _client_order_id: &str) -> Result<OrderStatus> {
            Err(unsupported())
        }

        #[cfg(feature = "ws")]
        async fn event_stream(&self) -> Result<Pin<Box<dyn Stream<Item = BrokerEvent> + Send>>> {
            Ok(Box::pin(futures_util::stream::empty()))
        }
    }

    #[tokio::test]
    async fn mock_methods_have_backward_compatible_default_errors() {
        // This implementation deliberately does not define either mock method.
        // If adding them to the trait breaks third-party implementations, this
        // test stops compiling before it can run.
        let client = ThirdPartyClient {
            config: ClientConfig::new("http://127.0.0.1:1"),
        };
        assert!(matches!(
            client.mock_account().await,
            Err(crate::Error::InvalidRequest(message)) if message == "mock account is not supported"
        ));
        assert!(matches!(
            client
                .init_mock_account(&crate::types::MockAccountInitRequest::default())
                .await,
            Err(crate::Error::InvalidRequest(message)) if message == "mock account is not supported"
        ));
    }
}
