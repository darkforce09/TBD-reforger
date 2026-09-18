//! Prometheus text exposition format 0.0.4 — rendering the registry, and serving `GET /metrics`.
//!
//! The registry itself is I/O-free: values it cannot accumulate (a live database ping, the
//! connection-pool depth) are sampled here and passed in as a [`Scrape`].

use std::fmt::Write as _;
use std::sync::atomic::Ordering;
use std::time::Duration;

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use sqlx::PgPool;

use super::health_probe::probe_db;
use super::metrics_registry::{LATENCY_BUCKETS_S, Registry};

/// Prometheus text exposition content type (format version 0.0.4).
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Runtime values sampled at scrape time rather than accumulated (pool depth, a live
/// database ping). Passed to [`Registry::render`] so the registry stays I/O-free.
#[derive(Debug, Clone, Copy)]
pub struct Scrape {
    /// 1 when a `SELECT 1` round-tripped within the probe budget, else 0.
    pub db_up: bool,
    /// How long that ping took. Meaningless when `db_up` is false.
    pub db_ping: Duration,
    /// `PgPool::size()` — connections currently owned by the pool.
    pub pool_connections: u32,
    /// `PgPool::num_idle()` — of those, how many are free.
    pub pool_idle: usize,
}

impl Registry {
    /// Render the whole registry in Prometheus text exposition format 0.0.4.
    pub fn render(&self, s: &Scrape) -> String {
        let t = self.read();
        let mut out = String::with_capacity(4096);

        out.push_str("# HELP tbd_build_info Build metadata for the running API; always 1.\n");
        out.push_str("# TYPE tbd_build_info gauge\n");
        let _ = writeln!(
            out,
            "tbd_build_info{{version=\"{}\"}} 1",
            esc(env!("CARGO_PKG_VERSION"))
        );

        out.push_str("# HELP tbd_process_start_time_seconds Unix start time of this router.\n");
        out.push_str("# TYPE tbd_process_start_time_seconds gauge\n");
        let _ = writeln!(out, "tbd_process_start_time_seconds {}", self.start_unix_s);

        out.push_str("# HELP tbd_uptime_seconds Seconds since this router was assembled.\n");
        out.push_str("# TYPE tbd_uptime_seconds gauge\n");
        let _ = writeln!(out, "tbd_uptime_seconds {:.3}", self.uptime().as_secs_f64());

        out.push_str("# HELP tbd_http_requests_in_flight Requests currently being served.\n");
        out.push_str("# TYPE tbd_http_requests_in_flight gauge\n");
        let _ = writeln!(out, "tbd_http_requests_in_flight {}", self.in_flight());

        out.push_str("# HELP tbd_http_requests_total Completed HTTP requests.\n");
        out.push_str("# TYPE tbd_http_requests_total counter\n");
        for ((method, route, status), c) in &t.requests {
            let _ = writeln!(
                out,
                "tbd_http_requests_total{{method=\"{}\",route=\"{}\",status=\"{}\"}} {}",
                esc(method),
                esc(route),
                status,
                c.load(Ordering::Relaxed)
            );
        }

        out.push_str("# HELP tbd_http_request_duration_seconds Request latency.\n");
        out.push_str("# TYPE tbd_http_request_duration_seconds histogram\n");
        for ((method, route), h) in &t.latency {
            let (m, r) = (esc(method), esc(route));
            for (slot, upper) in h.buckets.iter().zip(LATENCY_BUCKETS_S) {
                let _ = writeln!(
                    out,
                    "tbd_http_request_duration_seconds_bucket{{method=\"{m}\",route=\"{r}\",le=\"{upper}\"}} {}",
                    slot.load(Ordering::Relaxed)
                );
            }
            let count = h.count.load(Ordering::Relaxed);
            let _ = writeln!(
                out,
                "tbd_http_request_duration_seconds_bucket{{method=\"{m}\",route=\"{r}\",le=\"+Inf\"}} {count}"
            );
            let _ = writeln!(
                out,
                "tbd_http_request_duration_seconds_sum{{method=\"{m}\",route=\"{r}\"}} {:.6}",
                h.sum_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0
            );
            let _ = writeln!(
                out,
                "tbd_http_request_duration_seconds_count{{method=\"{m}\",route=\"{r}\"}} {count}"
            );
        }

        out.push_str(
            "# HELP tbd_http_rate_limited_total Requests refused by the rate limiter (429).\n",
        );
        out.push_str("# TYPE tbd_http_rate_limited_total counter\n");
        for (route, c) in &t.limited {
            let _ = writeln!(
                out,
                "tbd_http_rate_limited_total{{route=\"{}\"}} {}",
                esc(route),
                c.load(Ordering::Relaxed)
            );
        }

        out.push_str("# HELP tbd_db_up 1 when the database answered SELECT 1 at scrape.\n");
        out.push_str("# TYPE tbd_db_up gauge\n");
        let _ = writeln!(out, "tbd_db_up {}", u8::from(s.db_up));

        out.push_str("# HELP tbd_db_ping_seconds Duration of the scrape-time SELECT 1.\n");
        out.push_str("# TYPE tbd_db_ping_seconds gauge\n");
        let _ = writeln!(out, "tbd_db_ping_seconds {:.6}", s.db_ping.as_secs_f64());

        out.push_str("# HELP tbd_db_pool_connections sqlx pool connections by state.\n");
        out.push_str("# TYPE tbd_db_pool_connections gauge\n");
        let idle = s.pool_idle as u64;
        let in_use = u64::from(s.pool_connections).saturating_sub(idle);
        let _ = writeln!(out, "tbd_db_pool_connections{{state=\"idle\"}} {idle}");
        let _ = writeln!(out, "tbd_db_pool_connections{{state=\"in_use\"}} {in_use}");

        out.push_str(
            "# HELP tbd_metrics_series_dropped_total Samples dropped at the cardinality cap.\n",
        );
        out.push_str("# TYPE tbd_metrics_series_dropped_total counter\n");
        let _ = writeln!(
            out,
            "tbd_metrics_series_dropped_total {}",
            self.dropped_series()
        );

        out
    }
}

/// Escape a Prometheus label value (`\`, `"`, newline). Our labels are HTTP methods
/// and route templates and contain none of these — this exists so that stays true by
/// enforcement rather than by assumption.
fn esc(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

/// `GET /metrics` — Prometheus text exposition. Service-token gated at the route.
pub(crate) async fn metrics_scrape(reg: &Registry, pool: &PgPool) -> Response {
    let (db_up, db_ping, _) = probe_db(pool).await;
    let body = reg.render(&Scrape {
        db_up,
        db_ping,
        pool_connections: pool.size(),
        pool_idle: pool.num_idle(),
    });
    (StatusCode::OK, [(header::CONTENT_TYPE, CONTENT_TYPE)], body).into_response()
}

#[cfg(test)]
#[path = "tests/metrics_exposition.rs"]
mod tests;
