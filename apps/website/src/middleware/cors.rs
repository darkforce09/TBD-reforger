//! CORS — Rust port of `cors.go`. Reflects an allow-listed Origin (never `*`, no
//! credentials — the API is bearer-authed), answers `OPTIONS` preflight with 204.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::state::AppState;

const ALLOW_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";
const ALLOW_HEADERS: &str = "Authorization, Content-Type, X-Service-Token, X-Request-ID";

pub async fn cors(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let is_preflight = req.method() == Method::OPTIONS;
    let allowed = origin
        .as_deref()
        .is_some_and(|o| state.cors_origins.contains(o.trim_end_matches('/')));

    // OPTIONS always short-circuits to 204 (matching Go); other methods run through.
    let mut resp = if is_preflight {
        Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .expect("empty 204 body")
    } else {
        next.run(req).await
    };

    if allowed
        && let Some(o) = origin
        && let Ok(ov) = HeaderValue::from_str(&o)
    {
        let h = resp.headers_mut();
        h.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, ov);
        h.insert(header::VARY, HeaderValue::from_static("Origin"));
        h.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(ALLOW_METHODS),
        );
        h.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(ALLOW_HEADERS),
        );
        h.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static("600"),
        );
    }
    resp
}
