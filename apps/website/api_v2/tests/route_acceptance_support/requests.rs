//! One request against the router, and the readers and checks of its response.
//!
//! **Role:** sends a request through `oneshot` from a fresh synthetic peer, reads the response
//! as bytes (JSON and binary) or as its first server-sent event, and checks the `{error,
//! details?}` refusal envelope.
//!
//! **Position:** test support; the dimension runner and the part worlds send through
//! [`send`]; nothing here knows a route.
//!
//! **Signals & state:** a process-wide peer counter, so every request keys its own rate-limit
//! bucket.
//!
//! **Invariants:** every request carries its own `ConnectInfo`, so no suite reaches the
//! per-address limiter; a 429 is a harness failure, never a result; a `text/event-stream` body is
//! read only up to its first complete event, within a timeout, because the stream does not end.

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use futures::StreamExt;
use serde_json::Value;
use tower::ServiceExt;

/// How long a stream may take to deliver its first event.
const FIRST_EVENT_TIMEOUT: Duration = Duration::from_secs(10);
/// The largest non-stream body read.
const MAX_BODY: usize = 256 << 20;

static PEER: AtomicU32 = AtomicU32::new(1);

/// A synthetic client address no other request of this process uses.
pub fn next_peer() -> SocketAddr {
    let n = PEER.fetch_add(1, Ordering::Relaxed);
    let [_, b, c, d] = n.to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

/// A request to send: method, URI, optional bearer, headers and body.
#[derive(Debug, Clone, Default)]
pub struct Outgoing {
    pub method: String,
    pub uri: String,
    pub bearer: Option<String>,
    pub headers: Vec<(String, String)>,
    /// The body bytes and their `Content-Type` (none when `None`).
    pub body: Option<(Vec<u8>, Option<String>)>,
}

impl Outgoing {
    /// A request with no bearer, headers or body.
    pub fn new(method: &str, uri: impl Into<String>) -> Outgoing {
        Outgoing {
            method: method.to_string(),
            uri: uri.into(),
            ..Outgoing::default()
        }
    }

    /// Add `Authorization: Bearer <token>`.
    pub fn bearer(mut self, token: impl Into<String>) -> Outgoing {
        self.bearer = Some(token.into());
        self
    }

    /// A JSON body with `Content-Type: application/json`.
    pub fn json(mut self, body: &Value) -> Outgoing {
        let bytes = serde_json::to_vec(body).expect("serialise request body");
        self.body = Some((bytes, Some("application/json".into())));
        self
    }
}

/// A read response.
#[derive(Debug, Clone)]
pub struct Received {
    pub status: StatusCode,
    pub headers: HeaderMap,
    /// The whole body, or for an event stream the bytes up to its first complete event.
    pub body: Vec<u8>,
}

impl Received {
    /// The body as JSON, when it is JSON.
    pub fn json(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }

    /// The `Content-Type` header, or `""`.
    pub fn content_type(&self) -> &str {
        self.header(header::CONTENT_TYPE.as_str()).unwrap_or("")
    }

    /// A header's value as text.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    /// Up to 600 characters of the body, for failure messages.
    pub fn excerpt(&self) -> String {
        let text = String::from_utf8_lossy(&self.body);
        let mut excerpt: String = text.chars().take(600).collect();
        if text.chars().count() > 600 {
            excerpt.push('…');
        }
        excerpt
    }

    /// The `data` of the first event, parsed as JSON.
    pub fn first_event_data(&self) -> Option<Value> {
        let text = String::from_utf8_lossy(&self.body);
        let frame = text.split("\n\n").next()?;
        let data: Vec<&str> = frame
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim_start)
            .collect();
        serde_json::from_str(&data.join("\n")).ok()
    }
}

/// Send `outgoing` through `app` from a fresh peer and read the response.
pub async fn send(app: &Router, outgoing: &Outgoing) -> Received {
    let method = Method::from_bytes(outgoing.method.as_bytes()).expect("a valid method");
    let mut builder = Request::builder().method(method).uri(&outgoing.uri);
    if let Some(token) = &outgoing.bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    for (name, value) in &outgoing.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let body = match &outgoing.body {
        Some((bytes, content_type)) => {
            if let Some(content_type) = content_type {
                builder = builder.header(header::CONTENT_TYPE, content_type.as_str());
            }
            Body::from(bytes.clone())
        }
        None => Body::empty(),
    };
    let mut request = builder.body(body).expect("build request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    let status = response.status();
    assert_ne!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "{} {} was rate limited before its handler ran: the per-peer key is not varying",
        outgoing.method,
        outgoing.uri
    );
    let headers = response.headers().clone();
    let is_stream = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/event-stream"));
    let body = if is_stream {
        read_first_event(response.into_body()).await
    } else {
        to_bytes(response.into_body(), MAX_BODY)
            .await
            .map(|bytes| bytes.to_vec())
            .unwrap_or_else(|error| format!("<body unreadable: {error}>").into_bytes())
    };
    Received {
        status,
        headers,
        body,
    }
}

/// The stream's bytes up to and including its first `\n\n`, or what arrived before the
/// timeout or the end of the stream.
pub async fn read_first_event(body: Body) -> Vec<u8> {
    let mut stream = body.into_data_stream();
    let mut buffer = Vec::new();
    let read = async {
        while let Some(chunk) = stream.next().await {
            let Ok(chunk) = chunk else { break };
            buffer.extend_from_slice(&chunk);
            if buffer.windows(2).any(|pair| pair == b"\n\n") {
                break;
            }
        }
    };
    let _ = tokio::time::timeout(FIRST_EVENT_TIMEOUT, read).await;
    buffer
}

/// `None` when the body is the refusal envelope `{"error": string, "details"?: any}` and
/// nothing else, else what is wrong with it.
pub fn envelope_problem(received: &Received) -> Option<String> {
    let Some(Value::Object(fields)) = received.json() else {
        return Some(format!(
            "the refusal is not a JSON object (Content-Type `{}`): {}",
            received.content_type(),
            received.excerpt()
        ));
    };
    if !fields.get("error").is_some_and(Value::is_string) {
        return Some(format!(
            "the refusal has no string `error`: {}",
            received.excerpt()
        ));
    }
    let extra: Vec<&String> = fields
        .keys()
        .filter(|key| !matches!(key.as_str(), "error" | "details"))
        .collect();
    (!extra.is_empty()).then(|| format!("the refusal carries keys beyond the envelope: {extra:?}"))
}
