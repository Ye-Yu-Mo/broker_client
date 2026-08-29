//! WebSocket event stream with automatic reconnection and resubscription.

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

use super::{QuoteSubscription, TwClient, TwEvent};
use crate::error::{Error, Result};

type InnerStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A single connected WebSocket stream.
#[derive(Debug)]
pub struct TwWebSocket {
    inner: InnerStream,
}

impl TwWebSocket {
    /// Sends a text message over the WebSocket.
    pub async fn send_text(&mut self, text: &str) -> Result<()> {
        self.inner
            .send(Message::Text(text.to_owned().into()))
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))
    }
}

impl Stream for TwWebSocket {
    type Item = TwEvent;

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

/// Parses a WebSocket text frame into a [`TwEvent`].
///
/// Malformed JSON is converted into an `Unknown` event instead of panicking or
/// terminating the stream.
fn parse_event(text: &str) -> TwEvent {
    serde_json::from_str::<TwEvent>(text).unwrap_or_else(|_| TwEvent::Unknown {
        type_name: "_malformed".to_owned(),
        data: Value::String(text.to_owned()),
    })
}

fn ws_url(client: &TwClient) -> String {
    let http_base = client
        .config()
        .ws_base_url
        .as_deref()
        .unwrap_or(&client.config().base_url);
    let base = http_base
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    format!("{}/ws", base.trim_end_matches('/'))
}

/// Establishes a single WebSocket connection.
pub(crate) async fn connect(client: &TwClient) -> Result<TwWebSocket> {
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
    Ok(TwWebSocket { inner })
}

/// An auto-reconnecting WebSocket event stream.
///
/// The stream reconnects after connection loss and re-sends every remembered
/// subscription (added through [`TwClient::subscribe_quotes`]).
pub struct TwEventStream {
    rx: mpsc::Receiver<TwEvent>,
}

impl Stream for TwEventStream {
    type Item = TwEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Creates an auto-reconnecting event stream.
pub(crate) fn event_stream(client: TwClient) -> Result<TwEventStream> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        run_event_loop(client, tx).await;
    });
    Ok(TwEventStream { rx })
}

async fn run_event_loop(client: TwClient, tx: mpsc::Sender<TwEvent>) {
    let max_attempts = client.config().ws_max_reconnect_attempts;
    let base_backoff_ms = client.config().ws_base_backoff_ms;
    let mut attempt = 0u32;

    loop {
        match connect(&client).await {
            Ok(mut ws) => {
                // Restore the remembered subscriptions through the documented
                // HTTP subscribe endpoint after every (re)connect.
                let subscriptions: Vec<QuoteSubscription> = client
                    .subscriptions
                    .lock()
                    .map(|guard| guard.clone())
                    .unwrap_or_default();
                for subscription in &subscriptions {
                    if let Err(err) = client.subscribe_quotes_http(subscription).await {
                        let _ = tx
                            .send(TwEvent::Unknown {
                                type_name: "_resubscribe_error".to_owned(),
                                data: Value::String(err.to_string()),
                            })
                            .await;
                        break;
                    }
                }

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
                    .send(TwEvent::Unknown {
                        type_name: "_connect_error".to_owned(),
                        data: Value::String(err.to_string()),
                    })
                    .await;
            }
        }

        if max_attempts > 0 && attempt >= max_attempts {
            let _ = tx
                .send(TwEvent::Unknown {
                    type_name: "_stream_terminated".to_owned(),
                    data: Value::String("maximum WebSocket reconnect attempts reached".to_owned()),
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
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::client::tw::{QuoteSubscription, QuoteType, TwClient};
    use crate::config::ClientConfig;

    fn test_client(uri: String) -> TwClient {
        TwClient::new(ClientConfig::new(uri))
    }

    #[tokio::test]
    async fn single_ws_connection_receives_events_in_order() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            ws.send(Message::Text(
                json!({"type": "welcome", "message": "connected"})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            ws.send(Message::Text(
                json!({"type": "order.updated", "data": {"client_order_id": "C1", "status": "FILLED"}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            ws.send(Message::Text(
                json!({
                    "type": "Login",
                    "data": {
                        "login": {"login_list": []},
                        "account": "S1",
                        "name": "測試",
                        "investor_id": "A1"
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            ws.send(Message::Text(
                json!({"type": "quote.updated", "data": {"stk_code": "2330"}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            ws.send(Message::Text(
                json!({"type": "heartbeat", "data": {}}).to_string().into(),
            ))
            .await
            .unwrap();
            ws.send(Message::Text(
                json!({"type": "future.event", "data": {"x": 1}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            // Keep the connection open briefly so the client can read all frames.
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let client = test_client(format!("http://{addr}"));
        let mut ws = client.connect_ws().await.unwrap();
        let first = ws.next().await.unwrap();
        assert!(matches!(first, TwEvent::Welcome { .. }));
        let second = ws.next().await.unwrap();
        assert!(matches!(second, TwEvent::OrderUpdated(_)));
        let third = ws.next().await.unwrap();
        assert!(matches!(third, TwEvent::Login(_)));
        let fourth = ws.next().await.unwrap();
        assert!(matches!(fourth, TwEvent::QuoteUpdated(_)));
        let fifth = ws.next().await.unwrap();
        assert!(matches!(fifth, TwEvent::Heartbeat(_)));
        let sixth = ws.next().await.unwrap();
        assert!(matches!(sixth, TwEvent::Unknown { type_name, .. } if type_name == "future.event"));

        server.await.unwrap();
    }

    #[test]
    fn malformed_json_is_reported_as_unknown_without_panicking() {
        let event = parse_event("this is not json");
        assert!(matches!(
            event,
            TwEvent::Unknown { type_name, data } if type_name == "_malformed" && data.is_string()
        ));
    }

    #[test]
    fn ws_url_uses_ws_without_token_query() {
        let client = TwClient::new(
            ClientConfig::new("http://127.0.0.1:8000")
                .token("abc/def g")
                .ws_base_url("http://127.0.0.1:9000/"),
        );
        let url = ws_url(&client);
        assert_eq!(url, "ws://127.0.0.1:9000/ws");
    }

    #[tokio::test]
    async fn event_stream_reconnects_and_resubscribes() {
        // HTTP subscribe is served by wiremock on one port; WebSocket is
        // served by a raw tokio listener on another port.
        let http_server = MockServer::start().await;
        let subscription = QuoteSubscription {
            r#type: QuoteType::FiveTick,
            symbols: vec!["2330".to_owned()],
            account: None,
            market_type: None,
            index_flag: None,
        };
        Mock::given(method("POST"))
            .and(path("/api/v1/quotes/subscribe"))
            .and(body_json(serde_json::to_value(&subscription).unwrap()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok"
            })))
            .mount(&http_server)
            .await;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            for connection in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = accept_async(stream).await.unwrap();

                if connection == 0 {
                    ws.send(Message::Text(
                        json!({"type": "welcome", "message": "connected"})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                    // Drop to force the client to reconnect.
                    drop(ws);
                } else {
                    ws.send(Message::Text(
                        json!({"type": "order.updated", "data": {"client_order_id": "C1", "status": "FILLED"}})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        });

        let config = ClientConfig::new(http_server.uri())
            .ws_base_url(format!("http://{addr}"))
            .ws_max_reconnect_attempts(5)
            .ws_base_backoff_ms(10);
        let client = TwClient::new(config);
        client
            .subscriptions
            .lock()
            .unwrap()
            .push(subscription.clone());

        let mut events = client.event_stream().await.unwrap();
        let first = events.next().await.unwrap();
        assert!(matches!(first, TwEvent::Welcome { .. }));
        let second = tokio::time::timeout(Duration::from_secs(2), events.next())
            .await
            .expect("timed out waiting for reconnect event")
            .unwrap();
        assert!(matches!(second, TwEvent::OrderUpdated(_)));

        server.await.unwrap();
        let requests = http_server.received_requests().await.unwrap();
        assert!(
            requests.len() >= 2,
            "expected at least one HTTP resubscribe per WebSocket connection"
        );
    }
}
