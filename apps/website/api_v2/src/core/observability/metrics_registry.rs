//! Prometheus metric accumulation: the families, their label keys, and the cardinality cap.
//!
//! # Why there is no metrics crate in `Cargo.toml`
//!
//! The obvious move is `metrics` + `metrics-exporter-prometheus`. It is not taken, and the
//! reasons are recorded so the trade can be re-made knowingly rather than re-argued:
//!
//! 1. **Nothing in this family is in `Cargo.lock`** — not `metrics`, not `prometheus`,
//!    not `opentelemetry`, not `sentry`. Adding one is not a version bump, it is a new
//!    subtree (`metrics-util` → `crossbeam-*`, `sketches-ddsketch`, `hashbrown`) and a
//!    lockfile edit. `Cargo.lock` is shared with `website-frontend`, which builds to
//!    `wasm32-unknown-unknown`; "the wasm/frontend build is unaffected" is *provably* true
//!    while neither the lockfile nor the dependency graph changes.
//! 2. **The surface actually needed is small and fully testable.** Counters, one
//!    histogram family, a handful of gauges, and the 0.0.4 text exposition — every line
//!    covered by the sibling tests, including the cardinality cap that a third-party
//!    recorder would not give us either.
//! 3. **Cardinality is the real risk, not arithmetic.** It is handled by construction: the
//!    `route` label is axum's `MatchedPath` template (`/missions/{id}`, never a UUID) plus
//!    [`Registry::MAX_SERIES`] as a hard backstop with its own
//!    `tbd_metrics_series_dropped_total` counter.
//!
//! A need for OTLP export, or for handler-level instrumentation from modules that do not
//! hold the `Arc<Registry>`, is the moment to take the dependency — and the natural home for
//! the handle is then an `AppState` field.

use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Number of latency histogram buckets (see [`LATENCY_BUCKETS_S`]).
pub const N_BUCKETS: usize = 12;

/// Histogram bucket upper bounds in **seconds**, ascending. Rendered cumulatively
/// (`le=`), as the exposition format requires, plus the implicit `+Inf` bucket.
pub const LATENCY_BUCKETS_S: [f64; N_BUCKETS] = [
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// `route` label for a request that matched no route template (404s, static
/// fallbacks). A literal, so an unrouted scan cannot mint one series per URL.
pub const UNMATCHED_ROUTE: &str = "<unmatched>";

/// The status a throttled request carries, mirrored into `tbd_http_rate_limited_total`.
const TOO_MANY_REQUESTS: u16 = 429;

/// A cumulative-bucket latency histogram. Buckets are incremented for every bound the
/// observation falls under, so rendering is a straight read with no prefix sum.
pub(super) struct Histogram {
    pub(super) buckets: [AtomicU64; N_BUCKETS],
    /// Sum in microseconds — integer atomics, converted to seconds at render.
    pub(super) sum_micros: AtomicU64,
    pub(super) count: AtomicU64,
}

impl Histogram {
    fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            sum_micros: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    fn observe(&self, secs: f64) {
        for (slot, upper) in self.buckets.iter().zip(LATENCY_BUCKETS_S) {
            if secs <= upper {
                slot.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.sum_micros
            .fetch_add((secs * 1_000_000.0) as u64, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Every label-keyed family, behind one lock. Values are atomics so the hot path only
/// needs a **shared** read lock; the write lock is taken exactly once per new series.
#[derive(Default)]
pub(super) struct Tables {
    /// `tbd_http_requests_total{method,route,status}`
    pub(super) requests: BTreeMap<(String, String, u16), AtomicU64>,
    /// `tbd_http_request_duration_seconds{method,route}`
    pub(super) latency: BTreeMap<(String, String), Histogram>,
    /// `tbd_http_rate_limited_total{route}`
    pub(super) limited: BTreeMap<String, AtomicU64>,
}

/// One registry per [`crate::core::http_router::router`] call.
///
/// Deliberately **not** a `static`: a process-global recorder makes every test that
/// asserts a count depend on which other tests ran first, which is precisely the
/// "green over something it never examined" shape this design exists to avoid. The
/// cost is that only code holding the `Arc` can record — see the module header.
pub struct Registry {
    start: Instant,
    pub(super) start_unix_s: u64,
    tables: RwLock<Tables>,
    in_flight: AtomicI64,
    dropped: AtomicU64,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Hard ceiling on distinct label-sets **per family**. `route` is already a
    /// bounded template set, so hitting this means something is minting labels;
    /// dropping the sample and counting the drop beats unbounded memory growth.
    pub const MAX_SERIES: usize = 1024;

    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            start_unix_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or_default(),
            tables: RwLock::new(Tables::default()),
            in_flight: AtomicI64::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    /// How long this router has been assembled.
    pub fn uptime(&self) -> Duration {
        self.start.elapsed()
    }

    /// Requests currently in the middleware stack.
    pub fn in_flight(&self) -> i64 {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Samples discarded because a family was already at [`Self::MAX_SERIES`].
    pub fn dropped_series(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Enter a request: bumps the in-flight gauge and returns a guard that decrements
    /// on drop. A guard rather than a matching call because the panic path unwinds
    /// through this middleware, and a leaked gauge climbs forever.
    pub fn enter(self: &std::sync::Arc<Self>) -> InFlightGuard {
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        InFlightGuard(self.clone())
    }

    /// Record one completed request.
    pub fn record(&self, method: &str, route: &str, status: u16, elapsed: Duration) {
        let req_key = (method.to_owned(), route.to_owned(), status);
        let lat_key = (method.to_owned(), route.to_owned());
        self.ensure_series(&req_key, &lat_key, route, status);

        let t = self.read();
        if let Some(c) = t.requests.get(&req_key) {
            c.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(h) = t.latency.get(&lat_key) {
            h.observe(elapsed.as_secs_f64());
        }
        if status == TOO_MANY_REQUESTS
            && let Some(l) = t.limited.get(route)
        {
            l.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Create any missing series, honouring [`Self::MAX_SERIES`]. Takes the write
    /// lock only when at least one family is genuinely missing its key.
    fn ensure_series(
        &self,
        req_key: &(String, String, u16),
        lat_key: &(String, String),
        route: &str,
        status: u16,
    ) {
        {
            let t = self.read();
            let limited_ok = status != TOO_MANY_REQUESTS || t.limited.contains_key(route);
            if t.requests.contains_key(req_key) && t.latency.contains_key(lat_key) && limited_ok {
                return;
            }
        }
        let mut t = self.write();
        let mut dropped = 0u64;
        if !t.requests.contains_key(req_key) {
            if t.requests.len() < Self::MAX_SERIES {
                t.requests.insert(req_key.clone(), AtomicU64::new(0));
            } else {
                dropped += 1;
            }
        }
        if !t.latency.contains_key(lat_key) {
            if t.latency.len() < Self::MAX_SERIES {
                t.latency.insert(lat_key.clone(), Histogram::new());
            } else {
                dropped += 1;
            }
        }
        if status == TOO_MANY_REQUESTS && !t.limited.contains_key(route) {
            if t.limited.len() < Self::MAX_SERIES {
                t.limited.insert(route.to_owned(), AtomicU64::new(0));
            } else {
                dropped += 1;
            }
        }
        if dropped > 0 {
            self.dropped.fetch_add(dropped, Ordering::Relaxed);
        }
    }

    /// Current value of `tbd_http_requests_total` for one label-set (test/assertion
    /// helper — the exposition text is the contract, this is the cheap read).
    pub fn requests_total(&self, method: &str, route: &str, status: u16) -> u64 {
        let key = (method.to_owned(), route.to_owned(), status);
        self.read()
            .requests
            .get(&key)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Current value of `tbd_http_rate_limited_total` for one route.
    pub fn rate_limited_total(&self, route: &str) -> u64 {
        self.read()
            .limited
            .get(route)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// A poisoned metrics lock must not take the API down — the data is a counter, not
    /// an invariant. Recover the guard and carry on.
    pub(super) fn read(&self) -> std::sync::RwLockReadGuard<'_, Tables> {
        self.tables.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Tables> {
        self.tables.write().unwrap_or_else(|e| e.into_inner())
    }
}

/// Decrements `tbd_http_requests_in_flight` on drop — including on unwind.
pub struct InFlightGuard(std::sync::Arc<Registry>);

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[path = "tests/metrics_registry.rs"]
mod tests;
