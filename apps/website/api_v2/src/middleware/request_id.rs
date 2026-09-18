//! Request-ID + access logging — Rust port of `requestid.go` (`RequestID` + `Logger`).

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

const HEADER: &str = "x-request-id";

/// Correlation id stored in request extensions for the logging layer to read.
#[derive(Clone)]
pub struct RequestId(pub String);

/// Assign a request id (honoring an inbound `X-Request-ID`) and echo it on the
/// response. Runs outermost so the id is available to every inner layer.
pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.extensions_mut().insert(RequestId(id.clone()));
    let mut resp = next.run(req).await;
    if let Ok(hv) = HeaderValue::from_str(&id) {
        resp.headers_mut().insert(HEADER, hv);
    }
    resp
}

/// One structured access-log line per request (method, path, status, latency, id).
/// Server-side observability only — not part of the wire contract.
pub async fn logging(req: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_default();
    let rid = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_default();

    let resp = next.run(req).await;

    tracing::info!(
        target: "access",
        rid = %rid,
        method = %method,
        path = %path,
        status = resp.status().as_u16(),
        elapsed_ms = start.elapsed().as_millis() as u64,
        "request",
    );
    resp
}
