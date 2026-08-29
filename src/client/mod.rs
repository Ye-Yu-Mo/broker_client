//! Server-specific client modules and the unified [`BrokerClient`] trait.
//!
//! Feature gates keep the two server clients isolated:
//! - `client-tw` enables the Taiwan (`TwClient`) module.
//! - `client-a` enables the A-share (`AClient`) module.

pub mod broker;

#[cfg(feature = "client-tw")]
pub mod tw;

#[cfg(feature = "client-a")]
pub mod a;

#[cfg(feature = "client-tw")]
pub use tw::TwClient;

#[cfg(feature = "client-a")]
pub use a::AClient;

pub use broker::BrokerClient;
