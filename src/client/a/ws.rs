//! WebSocket event stream with automatic reconnection for the A-share server.

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::{AClient, AEvent};
use crate::error::{Error, Result};

type InnerStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A single connected A-share WebSocket stream.
#[derive(Debug)]
pub struct AWebSocket {
    inner: InnerStream,
}

impl AWebSocket {
    /// Sends a text message over the WebSocket.
    pub async fn send_text(&mut self, text: &str) -> Result<()> {
        self.inner
            .send(Message::Text(text.to_owned().into()))
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))
    }
}

impl Stream for AWebSocket {
    type Item = AEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(Message::Text(text)))) => {
                    return Poll::Ready(Some(parse_event(&text)));
                }
                Poll::Ready(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
                Poll::Ready(Some(Ok(Message::Close(_)))) => return Poll::Ready(None),
                Poll::Ready(Some(Ok(_))) => continue,
                Poll::Ready(Some(Err(_))) => return Poll::Ready(None),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Parses a WebSocket text frame into an [`AEvent`].
///
/// Malformed JSON is converted into an `Unknown` event instead of panicking or
/// terminating the stream.
fn parse_event(text: &str) -> AEvent {
    serde_json::from_str::<AEvent>(text).unwrap_or_else(|_| AEvent::Unknown {
        type_name: "_malformed".to_owned(),
        data: Value::String(text.to_owned()),
        timestamp_ms: None,
    })
}

fn ws_url(client: &AClient) -> String {
    let http_base = client
        .config()
        .ws_base_url
        .as_deref()
        .unwrap_or(&client.config().base_url);
    let base = http_base
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    format!("{}/v1/ws", base.trim_end_matches('/'))
}

/// Establishes a single WebSocket connection.
pub(crate) async fn connect(client: &AClient) -> Result<AWebSocket> {
    let mut request = ws_url(client)
        .into_client_request()
        .map_err(|e| Error::WebSocket(e.to_string()))?;

    if let Some(token) = client.config().token.as_deref() {
        if !token.is_empty() {
            let value = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| Error::WebSocket(e.to_string()))?;
            request.headers_mut().insert("Authorization", value);
        }
    }

    let (inner, _) = connect_async(request)
        .await
        .map_err(|e: tokio_tungstenite::tungstenite::Error| Error::WebSocket(e.to_string()))?;
    Ok(AWebSocket { inner })
}

/// An auto-reconnecting A-share WebSocket event stream.
pub struct AEventStream {
    rx: mpsc::Receiver<AEvent>,
}

impl Stream for AEventStream {
    type Item = AEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Creates an auto-reconnecting event stream.
pub(crate) fn event_stream(client: AClient) -> Result<AEventStream> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        run_event_loop(client, tx).await;
    });
    Ok(AEventStream { rx })
}

async fn run_event_loop(client: AClient, tx: mpsc::Sender<AEvent>) {
    let max_attempts = client.config().ws_max_reconnect_attempts;
    let base_backoff_ms = client.config().ws_base_backoff_ms;
    let mut attempt = 0u32;

    loop {
        match connect(&client).await {
            Ok(mut ws) => {
                while let Some(event) = ws.next().await {
                    // Only a successfully received first event proves the
                    // connection is usable; reset the retry counter then.
                    attempt = 0;

                    if tx.send(event).await.is_err() {
                        return;
                    }
                }
            }
            Err(err) => {
                let _ = tx
                    .send(AEvent::Unknown {
                        type_name: "_connect_error".to_owned(),
                        data: Value::String(err.to_string()),
                        timestamp_ms: None,
                    })
                    .await;
            }
        }

        if max_attempts > 0 && attempt >= max_attempts {
            let _ = tx
                .send(AEvent::Unknown {
                    type_name: "_stream_terminated".to_owned(),
                    data: Value::String("maximum WebSocket reconnect attempts reached".to_owned()),
                    timestamp_ms: None,
                })
                .await;
            return;
        }

        attempt += 1;
        tokio::time::sleep(Duration::from_millis(base_backoff_ms * u64::from(attempt))).await;
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use serde_json::json;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use tokio_tungstenite::tungstenite::Message;

    use super::*;
    use crate::client::a::AClient;
    use crate::config::ClientConfig;

    fn test_client(uri: String) -> AClient {
        AClient::new(ClientConfig::new(uri))
    }

    #[test]
    fn ws_url_uses_v1_ws_without_token_query() {
        let client = AClient::new(
            ClientConfig::new("http://127.0.0.1:8787")
                .token("abc/def g")
                .ws_base_url("http://127.0.0.1:9000/"),
        );
        let url = ws_url(&client);
        assert_eq!(url, "ws://127.0.0.1:9000/v1/ws");
    }

    #[test]
    fn malformed_json_is_reported_as_unknown_without_panicking() {
        let event = parse_event("this is not json");
        assert!(matches!(
            event,
            AEvent::Unknown { type_name, data, .. } if type_name == "_malformed" && data.is_string()
        ));
    }

    #[tokio::test]
    async fn single_ws_connection_receives_all_documented_events_in_order() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            for type_name in [
                "order.updated",
                "position.changed",
                "account.changed",
                "account.balance_changed",
                "query.cache_hit",
                "replace.updated",
                "order.no_mapping",
                "order.manual_review",
                "risk.panic",
                "health.changed",
                "future.event",
            ] {
                ws.send(Message::Text(
                    json!({
                        "type": type_name,
                        "timestamp_ms": 1730000000000_i64,
                        "data": {"value": type_name}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        });

        let client = test_client(format!("http://{addr}"));
        let mut ws = client.connect_ws().await.unwrap();
        let mut seen = Vec::new();
        while let Some(event) = ws.next().await {
            match event {
                AEvent::Unknown { type_name, .. } if type_name == "future.event" => {
                    seen.push("future.event".to_owned());
                    break;
                }
                AEvent::OrderUpdated { .. } => seen.push("order.updated".to_owned()),
                AEvent::PositionChanged { .. } => seen.push("position.changed".to_owned()),
                AEvent::AccountChanged { .. } => seen.push("account.changed".to_owned()),
                AEvent::AccountBalanceChanged { .. } => {
                    seen.push("account.balance_changed".to_owned())
                }
                AEvent::QueryCacheHit { .. } => seen.push("query.cache_hit".to_owned()),
                AEvent::ReplaceUpdated { .. } => seen.push("replace.updated".to_owned()),
                AEvent::OrderNoMapping { .. } => seen.push("order.no_mapping".to_owned()),
                AEvent::OrderManualReview { .. } => seen.push("order.manual_review".to_owned()),
                AEvent::RiskPanic { .. } => seen.push("risk.panic".to_owned()),
                AEvent::HealthChanged { .. } => seen.push("health.changed".to_owned()),
                AEvent::Unknown { .. } => seen.push("unknown".to_owned()),
            }
        }
        assert_eq!(
            seen,
            [
                "order.updated",
                "position.changed",
                "account.changed",
                "account.balance_changed",
                "query.cache_hit",
                "replace.updated",
                "order.no_mapping",
                "order.manual_review",
                "risk.panic",
                "health.changed",
                "future.event"
            ]
        );

        server.await.unwrap();
    }

    #[tokio::test]
    async fn event_stream_reconnects_after_drop() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            for connection in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = accept_async(stream).await.unwrap();
                if connection == 0 {
                    ws.send(Message::Text(
                        json!({"type": "order.updated", "data": {"seq": 1}})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                    drop(ws);
                } else {
                    ws.send(Message::Text(
                        json!({"type": "order.updated", "data": {"seq": 2}})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        });

        let client = AClient::new(
            ClientConfig::new("http://127.0.0.1:1")
                .ws_base_url(format!("http://{addr}"))
                .ws_max_reconnect_attempts(5)
                .ws_base_backoff_ms(10),
        );
        let mut events = client.event_stream().await.unwrap();
        let first = events.next().await.unwrap();
        assert!(matches!(first, AEvent::OrderUpdated { .. }));
        let second = tokio::time::timeout(Duration::from_secs(2), events.next())
            .await
            .expect("timed out waiting for reconnect event")
            .unwrap();
        assert!(matches!(second, AEvent::OrderUpdated { .. }));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn trait_event_stream_yields_unified_broker_events() {
        use crate::client::broker::BrokerClient;
        use crate::types::BrokerEvent;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            ws.send(Message::Text(
                json!({"type": "order.updated", "data": {"client_order_id": "C1"}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let client = AClient::new(
            ClientConfig::new("http://127.0.0.1:1")
                .ws_base_url(format!("http://{addr}"))
                .ws_max_reconnect_attempts(1)
                .ws_base_backoff_ms(10),
        );
        let client: Box<dyn BrokerClient> = Box::new(client);
        let mut events = client.event_stream().await.unwrap();
        let event = tokio::time::timeout(Duration::from_secs(2), events.next())
            .await
            .expect("timed out waiting for unified A event")
            .unwrap();
        assert!(matches!(event, BrokerEvent::OrderUpdated { .. }));

        server.await.unwrap();
    }
}
