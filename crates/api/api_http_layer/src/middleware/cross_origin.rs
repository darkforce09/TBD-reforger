//! CORS: reflects an allow-listed `Origin` (never `*`, and never with credentials — the API is
//! bearer-authed), and answers every `OPTIONS` preflight with 204.
//!
//! The middleware reads only its [`CorsOrigins`] sub-state, which axum extracts from the router
//! state through `FromRef`, so this module names no application state type.

use std::collections::HashSet;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

const ALLOW_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";
const ALLOW_HEADERS: &str = "Authorization, Content-Type, X-Request-ID";

/// The CORS allow-list: the configured origins with any trailing `/` trimmed, so `Origin`
/// headers compare equal whether or not an operator wrote the slash.
#[derive(Clone, Debug, Default)]
pub struct CorsOrigins(Arc<HashSet<String>>);

impl CorsOrigins {
    /// The allow-list of the configured `origins`, each normalised by trimming trailing `/`.
    pub fn from_configured(origins: &[String]) -> Self {
        Self(Arc::new(
            origins
                .iter()
                .map(|origin| origin.trim_end_matches('/').to_string())
                .collect(),
        ))
    }

    /// Whether `origin` (trailing `/` ignored) is on the allow-list.
    pub fn allows(&self, origin: &str) -> bool {
        self.0.contains(origin.trim_end_matches('/'))
    }
}

/// Reflects an allow-listed `Origin` and short-circuits every `OPTIONS` preflight with 204.
pub async fn cors(State(origins): State<CorsOrigins>, req: Request, next: Next) -> Response {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let is_preflight = req.method() == Method::OPTIONS;
    let allowed = origin.as_deref().is_some_and(|o| origins.allows(o));

    // OPTIONS always short-circuits to 204; other methods run through the inner stack.
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
