//! Authentication configuration and header injection.

use std::fmt;

/// Supported authentication header styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMethod {
    /// `Authorization: Bearer <token>`
    #[default]
    Bearer,
    /// `X-Auth-Token: <token>`
    XAuthToken,
}

impl AuthMethod {
    /// Returns the header name used by this authentication style.
    pub fn header_name(self) -> &'static str {
        match self {
            AuthMethod::Bearer => "authorization",
            AuthMethod::XAuthToken => "x-auth-token",
        }
    }

    /// Builds the header value for a token.
    pub fn header_value(self, token: &str) -> String {
        match self {
            AuthMethod::Bearer => format!("Bearer {token}"),
            AuthMethod::XAuthToken => token.to_owned(),
        }
    }
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMethod::Bearer => f.write_str("Bearer"),
            AuthMethod::XAuthToken => f.write_str("XAuthToken"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AuthMethod;

    #[test]
    fn bearer_uses_authorization_header() {
        assert_eq!(AuthMethod::Bearer.header_name(), "authorization");
        assert_eq!(AuthMethod::Bearer.header_value("secret"), "Bearer secret");
    }

    #[test]
    fn x_auth_token_uses_dedicated_header() {
        assert_eq!(AuthMethod::XAuthToken.header_name(), "x-auth-token");
        assert_eq!(AuthMethod::XAuthToken.header_value("secret"), "secret");
    }

    #[test]
    fn default_auth_method_is_bearer() {
        assert_eq!(AuthMethod::default(), AuthMethod::Bearer);
    }
}
