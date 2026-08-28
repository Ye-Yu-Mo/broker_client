//! Unified error type for the broker clients.

use std::fmt;

use serde_json::Value;

/// Convenience result alias used across the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur while talking to either server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Low-level connection/protocol failure.
    Transport(String),
    /// Request timed out.
    Timeout,
    /// HTTP response with a non-2xx status that could not be mapped to an API error.
    Http {
        /// HTTP status code.
        status: u16,
        /// Raw response body (possibly empty).
        body: String,
    },
    /// Server-side API error, including TW and A-share error envelopes.
    Api {
        /// Server error code (string in both documented formats).
        code: String,
        /// Human-readable message.
        message: String,
        /// Extra structured detail.
        detail: Value,
    },
    /// JSON decoding failed.
    Decode {
        /// Raw body that failed to decode.
        body: String,
        /// Human-readable serde error.
        source: String,
    },
    /// WebSocket related error.
    WebSocket(String),
    /// Invalid URL or URL composition error.
    InvalidUrl(String),
    /// Client-side validation failed before a request was sent.
    InvalidRequest(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Transport(msg) => write!(f, "transport error: {msg}"),
            Error::Timeout => f.write_str("request timed out"),
            Error::Http { status, body } => {
                write!(f, "http error {status}: {body}")
            }
            Error::Api {
                code,
                message,
                detail,
            } => {
                write!(f, "api error {code}: {message} (detail: {detail})")
            }
            Error::Decode { body, source } => {
                write!(f, "failed to decode response: {source}; body: {body}")
            }
            Error::WebSocket(msg) => write!(f, "websocket error: {msg}"),
            Error::InvalidUrl(url) => write!(f, "invalid url: {url}"),
            Error::InvalidRequest(msg) => write!(f, "invalid request: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            Error::Timeout
        } else {
            Error::Transport(value.to_string())
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Decode {
            body: String::new(),
            source: value.to_string(),
        }
    }
}

impl Error {
    /// Creates a [`Error::Decode`] preserving the raw body.
    pub fn decode(body: String, source: serde_json::Error) -> Self {
        Error::Decode {
            body,
            source: source.to_string(),
        }
    }

    /// Returns true for errors that are safe to retry for idempotent GETs.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Transport(_) | Error::Timeout => true,
            Error::Http { status, .. } => *status == 429 || *status >= 500,
            Error::Api { .. }
            | Error::Decode { .. }
            | Error::WebSocket(_)
            | Error::InvalidUrl(_)
            | Error::InvalidRequest(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Error;

    #[test]
    fn timeout_display_is_explicit() {
        assert_eq!(Error::Timeout.to_string(), "request timed out");
    }

    #[test]
    fn http_error_keeps_status_and_body() {
        let err = Error::Http {
            status: 404,
            body: "not found".to_owned(),
        };
        assert_eq!(err.to_string(), "http error 404: not found");
        assert!(matches!(
            err,
            Error::Http { status: 404, body } if body == "not found"
        ));
    }

    #[test]
    fn api_error_keeps_code_message_detail() {
        let err = Error::Api {
            code: "ORDER_NOT_FOUND".to_owned(),
            message: "missing".to_owned(),
            detail: json!({"id": "x"}),
        };
        assert!(err.to_string().contains("ORDER_NOT_FOUND"));
    }

    #[test]
    fn invalid_request_is_not_retryable_and_has_clear_message() {
        let err = Error::InvalidRequest("replace must set new_price and/or new_quantity".into());
        assert!(err.to_string().contains("invalid request"));
        assert!(!err.is_retryable());
    }

    #[test]
    fn retryable_statuses_are_identified() {
        assert!(Error::Timeout.is_retryable());
        assert!(Error::Transport("boom".into()).is_retryable());
        assert!(
            Error::Http {
                status: 429,
                body: String::new()
            }
            .is_retryable()
        );
        assert!(
            Error::Http {
                status: 503,
                body: String::new()
            }
            .is_retryable()
        );
        assert!(
            !Error::Http {
                status: 401,
                body: String::new()
            }
            .is_retryable()
        );
        assert!(
            !Error::Api {
                code: "X".into(),
                message: "y".into(),
                detail: serde_json::Value::Null
            }
            .is_retryable()
        );
    }
}
