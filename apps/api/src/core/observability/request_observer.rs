//! The middleware that feeds the metrics registry.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;

use super::metrics_registry::{Registry, UNMATCHED_ROUTE};

/// Count every request: `tbd_http_requests_total`, the latency histogram, the in-flight
/// gauge, and `tbd_http_rate_limited_total` on a 429.
///
/// Mounted **outside** the panic-catcher and the rate limiter so a 500-from-panic and a
/// 429-from-throttle are both counted; see [`crate::core::http_router::router`] for the layer
/// order that guarantees it.
///
/// The `route` label is axum's [`MatchedPath`] — the registered template, so
/// `/api/v1/missions/{id}` is one series and not one per mission UUID. Requests that match
/// nothing collapse to [`UNMATCHED_ROUTE`].
pub(crate) async fn observe(
    State(reg): State<Arc<Registry>>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().as_str().to_owned();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| UNMATCHED_ROUTE.to_owned());

    let _in_flight = reg.enter();
    let started = Instant::now();
    let resp = next.run(req).await;
    reg.record(&method, &route, resp.status().as_u16(), started.elapsed());
    resp
}
