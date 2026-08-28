//! Server-specific client modules.
//!
//! Feature gates keep the two server clients isolated:
//! - `client-tw` enables the Taiwan (`TwClient`) module.
//! - `client-a` enables the A-share (`AClient`) module.

#[cfg(feature = "client-tw")]
pub mod tw;

#[cfg(feature = "client-a")]
pub mod a;

#[cfg(feature = "client-tw")]
pub use tw::TwClient;

#[cfg(feature = "client-a")]
pub use a::AClient;
