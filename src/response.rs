//! Response envelope parsing for the two documented server styles.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

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

/// Parses a TW error body into an [`Error::Api`] if the shape matches.
pub fn parse_tw_error_body(body: &str) -> Option<Error> {
    serde_json::from_str::<TwErrorEnvelope>(body)
        .ok()
        .map(|envelope| Error::Api {
            code: envelope.detail.code,
            message: envelope.detail.message,
            detail: envelope.detail.detail,
        })
}

/// Parses an A-share error body into an [`Error::Api`] if the shape matches.
pub fn parse_a_error_body(body: &str) -> Option<Error> {
    let parsed: ApiErrorBody = serde_json::from_str(body).ok()?;
    // A successful response could coincidentally have these fields; this
    // helper is only called for non-2xx responses, so any matching object is
    // treated as a documented API error.
    Some(Error::Api {
        code: parsed.code,
        message: parsed.message,
        detail: parsed.detail,
    })
}

/// Converts an HTTP error status/body into the most specific [`Error`].
///
/// It first tries the TW failure envelope, then the A-share error body.
/// Unknown bodies become a plain [`Error::Http`] so raw data is preserved.
pub fn http_error(status: u16, body: String) -> Error {
    if let Some(err) = parse_tw_error_body(&body) {
        return err;
    }
    if let Some(err) = parse_a_error_body(&body) {
        return err;
    }
    Error::Http { status, body }
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
            Error::Api { code, message, detail } if code == "RATE_LIMITED" && message == "slow down" && detail == json!({"hint": 1})
        ));
    }

    #[test]
    fn a_error_body_is_recognized() {
        let raw = r#"{"code":"error","message":"bad","detail":{"field":"x"}}"#;
        let err = parse_a_error_body(raw).unwrap();
        assert!(matches!(
            err,
            Error::Api { code, message, detail } if code == "error" && message == "bad" && detail == json!({"field": "x"})
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
}
