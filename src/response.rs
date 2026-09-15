//! Response envelope parsing for the two documented server styles.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::error::{Error, Result};

/// TW success envelope: `{ "code": 0, "message": "ok", "data": ... }`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TwEnvelope<T> {
    /// `0` means success.
    pub code: i64,
    /// Server-provided message.
    pub message: String,
    /// Success payload; absent/`null` on some endpoints.
    pub data: Option<T>,
}

impl<T> TwEnvelope<T>
where
    T: DeserializeOwned,
{
    /// Parses a TW envelope from a JSON string.
    pub fn from_json(body: &str) -> Result<Self> {
        serde_json::from_str(body).map_err(|e| Error::decode(body.to_owned(), e))
    }

    /// Converts the envelope into the typed payload, mapping non-zero codes to [`Error::Api`].
    pub fn into_data(self) -> Result<T> {
        if self.code == 0 {
            self.data.ok_or_else(|| Error::Decode {
                body: "missing data field in TW success envelope".to_owned(),
                source: "expected data".to_owned(),
            })
        } else {
            Err(Error::Api {
                code: self.code.to_string(),
                message: self.message,
                detail: Value::Null,
                status: None,
            })
        }
    }

    /// Checks that the envelope succeeded and ignores the payload.
    ///
    /// This is useful for endpoints whose success response has no `data` field.
    pub fn into_unit(self) -> Result<()> {
        if self.code == 0 {
            Ok(())
        } else {
            Err(Error::Api {
                code: self.code.to_string(),
                message: self.message,
                detail: Value::Null,
                status: None,
            })
        }
    }
}

/// TW failure envelope: `{ "detail": { "code": ..., "message": ..., "detail": ... } }`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TwErrorEnvelope {
    /// Wrapped error details.
    pub detail: ApiErrorBody,
}

/// A-share error body: `{ "code": "error", "message": "...", "detail": {} }`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ApiErrorBody {
    /// Server error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional structured detail.
    #[serde(default)]
    pub detail: Value,
}

/// A-share bare error body: `{ "error": "..." }`.
///
/// This is the shape handlers produce before the `/v1` middleware rewrites it
/// into [`ApiErrorBody`].
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BareErrorBody {
    /// Server-provided message.
    pub error: String,
}

/// Parses a TW error body into an [`Error::Api`] if the shape matches.
pub fn parse_tw_error_body(body: &str) -> Option<Error> {
    serde_json::from_str::<TwErrorEnvelope>(body)
        .ok()
        .map(|envelope| Error::Api {
            code: envelope.detail.code,
            message: envelope.detail.message,
            detail: envelope.detail.detail,
            status: None,
        })
}

/// Parses an A-share error body into an [`Error::Api`] if the shape matches.
///
/// The `/v1` group rewrites `{ "error": ... }` into
/// `{ "code", "message", "detail" }`, but the pre-rewrite shape is mapped the
/// same way so a route without that middleware cannot degrade into an opaque
/// [`Error::Http`] that hides the server's message.
pub fn parse_a_error_body(body: &str) -> Option<Error> {
    if let Ok(parsed) = serde_json::from_str::<ApiErrorBody>(body) {
        // A successful response could coincidentally have these fields; this
        // helper is only called for non-2xx responses, so any matching object
        // is treated as a documented API error.
        return Some(Error::Api {
            code: parsed.code,
            message: parsed.message,
            detail: parsed.detail,
            status: None,
        });
    }

    let bare: BareErrorBody = serde_json::from_str(body).ok()?;
    Some(Error::Api {
        code: "error".to_owned(),
        message: bare.error,
        detail: json!({}),
        status: None,
    })
}

/// Converts an HTTP error status/body into the most specific [`Error`].
///
/// It first tries the TW failure envelope, then the A-share error body. API
/// errors retain the HTTP status so GET retry policy can still distinguish 429
/// and 5xx responses. Unknown bodies become a plain [`Error::Http`] so raw data
/// is preserved.
pub fn http_error(status: u16, body: String) -> Error {
    if let Some(err) = parse_tw_error_body(&body) {
        return attach_http_status(err, status);
    }
    if let Some(err) = parse_a_error_body(&body) {
        return attach_http_status(err, status);
    }
    Error::Http { status, body }
}

fn attach_http_status(error: Error, status: u16) -> Error {
    match error {
        Error::Api {
            code,
            message,
            detail,
            ..
        } => Error::Api {
            code,
            message,
            detail,
            status: Some(status),
        },
        error => error,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{TwEnvelope, http_error, parse_a_error_body, parse_tw_error_body};
    use crate::error::Error;

    #[derive(Debug, serde::Deserialize, PartialEq, Eq)]
    struct Health {
        status: String,
    }

    #[test]
    fn tw_success_envelope_decodes_data() {
        let raw = r#"{"code":0,"message":"ok","data":{"status":"ok"}}"#;
        let envelope: TwEnvelope<Health> = TwEnvelope::from_json(raw).unwrap();
        assert_eq!(
            envelope.into_data().unwrap(),
            Health {
                status: "ok".to_owned()
            }
        );
    }

    #[test]
    fn tw_nonzero_code_maps_to_api_error() {
        let raw = r#"{"code":404,"message":"ORDER_NOT_FOUND","data":null}"#;
        let envelope: TwEnvelope<Health> = TwEnvelope::from_json(raw).unwrap();
        let err = envelope.into_data().unwrap_err();
        assert!(matches!(
            err,
            Error::Api { code, message, .. } if code == "404" && message == "ORDER_NOT_FOUND"
        ));
    }

    #[test]
    fn tw_error_envelope_is_recognized() {
        let raw = r#"{"detail":{"code":"RATE_LIMITED","message":"slow down","detail":{"hint":1}}}"#;
        let err = parse_tw_error_body(raw).unwrap();
        assert!(matches!(
            err,
            Error::Api { code, message, detail, .. } if code == "RATE_LIMITED" && message == "slow down" && detail == json!({"hint": 1})
        ));
    }

    #[test]
    fn a_error_body_is_recognized() {
        let raw = r#"{"code":"error","message":"bad","detail":{"field":"x"}}"#;
        let err = parse_a_error_body(raw).unwrap();
        assert!(matches!(
            err,
            Error::Api { code, message, detail, .. } if code == "error" && message == "bad" && detail == json!({"field": "x"})
        ));
    }

    #[test]
    fn unknown_error_body_becomes_http_error() {
        let err = http_error(502, "plain text".to_owned());
        assert!(matches!(
            err,
            Error::Http { status: 502, body } if body == "plain text"
        ));
    }

    #[test]
    fn common_http_statuses_map_to_http_error_without_api_body() {
        for status in [401, 404, 429, 503] {
            let err = http_error(status, "raw body".to_owned());
            assert!(matches!(
                err,
                Error::Http { status: s, body } if s == status && body == "raw body"
            ));
        }
    }

    #[test]
    fn bare_error_body_maps_to_api_error_like_the_v1_middleware() {
        // The A server's `v1_error_middleware` rewrites `{"error": msg}` into
        // `{code,message,detail}`. Mapping the pre-rewrite shape identically
        // keeps routes without that middleware from degrading into an opaque
        // `Error::Http` that hides the server's message.
        let err = http_error(404, r#"{"error": "mock 账户尚未初始化"}"#.to_owned());
        assert!(matches!(
            err,
            Error::Api {
                code,
                message,
                detail,
                status,
            } if code == "error"
                && message == "mock 账户尚未初始化"
                && detail == json!({})
                && status == Some(404)
        ));
    }
}
