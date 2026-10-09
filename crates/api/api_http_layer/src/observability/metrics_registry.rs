//! Prometheus metric accumulation: the families, their label keys, and the cardinality cap.
//!
//! **Role:** accumulates every metric the API exposes in one [`Registry`]: the HTTP families and
//! the Discord membership reconciliation outcomes.
//! **Position:** `api_http_layer::observability`; the registry is a field of
//! `api_state`'s `AppState`, fed through that state by the `observe`
//! middleware and by the Discord membership reconciliation
//! (`api_identity_and_access::services::discord_rest_reconciliation`), and read by
//! [`crate::observability::metrics_exposition`].
//! **Signals & state:** per [`Registry`], atomics behind one `RwLock` for the HTTP families and a
//! fixed array of atomics for the reconciliation outcomes; no process-global state.
//! **Invariants:** a [`Registry`] family never holds more than [`Registry::MAX_SERIES`] series;
//! the Discord outcome family holds exactly one series per [`DiscordReconcileOutcome`].
//!
//! # Why there is no metrics crate in `Cargo.toml`
//!
//! The obvious move is `metrics` + `metrics-exporter-prometheus`. It is not taken, and the
//! reasons are recorded so the trade can be re-made knowingly rather than re-argued:
//!
//! 1. **Nothing in this family is in `Cargo.lock`** — not `metrics`, not `prometheus`,
//!    not `opentelemetry`, not `sentry`. Adding one is not a version bump, it is a new
//!    subtree (`metrics-util` → `crossbeam-*`, `sketches-ddsketch`, `hashbrown`) and a
//!    lockfile edit. `Cargo.lock` is shared with `frontend`, which builds to
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
//! Every recorder reaches the registry through the application state it already holds: the
//! `observe` middleware through the router, and the Discord membership reconciliation, which
//! records outside the request path, through the state its worker receives. A need for OTLP
//! export, or for instrumentation from code that holds no application state, is the moment to
//! take the dependency.

use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use time_source::{Clock, PlatformClock};

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

/// Number of [`DiscordReconcileOutcome`] variants, and of `tbd_discord_reconcile_outcomes_total`
/// series.
pub const DISCORD_RECONCILE_OUTCOME_COUNT: usize = 5;

/// How one Discord membership reconciliation request ended: the `outcome` label of
/// `tbd_discord_reconcile_outcomes_total` and the `outcome` field of the `discord_reconciliation`
/// log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscordReconcileOutcome {
    /// Discord returned the member, and the membership was recorded under the lease.
    Member,
    /// Discord answered Unknown Member, and the non-membership was recorded under the lease.
    Nonmember,
    /// Discord answered, but the lease had lapsed or a newer claim held it, so the answer was
    /// discarded unrecorded.
    LeaseLost,
    /// Discord answered 429: the shared request schedule and the snapshot back off.
    RateLimited,
    /// Discord gave no usable answer (a transport failure, an unconfigured bot, a non-success
    /// status or a malformed body): the recorded membership stays and `last_error` names why.
    Unavailable,
}

impl DiscordReconcileOutcome {
    /// Every outcome in exposition order; each variant's discriminant is its index here and its
    /// slot in [`DiscordReconcileOutcomeCounts`].
    pub const ALL: [Self; DISCORD_RECONCILE_OUTCOME_COUNT] = [
        Self::Member,
        Self::Nonmember,
        Self::LeaseLost,
        Self::RateLimited,
        Self::Unavailable,
    ];

    /// The label value: `member`, `nonmember`, `lease_lost`, `rate_limited` or `unavailable`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Nonmember => "nonmember",
            Self::LeaseLost => "lease_lost",
            Self::RateLimited => "rate_limited",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One counter per [`DiscordReconcileOutcome`], behind `tbd_discord_reconcile_outcomes_total`.
pub struct DiscordReconcileOutcomeCounts([AtomicU64; DISCORD_RECONCILE_OUTCOME_COUNT]);

impl Default for DiscordReconcileOutcomeCounts {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordReconcileOutcomeCounts {
    /// Every counter at zero.
    pub const fn new() -> Self {
        Self([const { AtomicU64::new(0) }; DISCORD_RECONCILE_OUTCOME_COUNT])
    }

    /// Count one reconciliation request that ended in `outcome`.
    pub fn record(&self, outcome: DiscordReconcileOutcome) {
        self.0[outcome as usize].fetch_add(1, Ordering::Relaxed);
    }

    /// How many reconciliation requests ended in `outcome`.
    pub fn count(&self, outcome: DiscordReconcileOutcome) -> u64 {
        self.0[outcome as usize].load(Ordering::Relaxed)
    }
}

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

/// The API's metrics: one registry per API `AppState`, shared
/// through that state by the router's `observe` middleware, `/metrics`, `/healthz` and the
/// background workers.
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
    /// `tbd_discord_reconcile_outcomes_total{outcome}`
    discord_reconcile_outcomes: DiscordReconcileOutcomeCounts,
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

    /// An empty registry whose uptime starts now.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            start_unix_s: PlatformClock.now_unix_ms() / 1000,
            tables: RwLock::new(Tables::default()),
            in_flight: AtomicI64::new(0),
            dropped: AtomicU64::new(0),
            discord_reconcile_outcomes: DiscordReconcileOutcomeCounts::new(),
        }
    }

    /// How long this registry has existed: since its application state was built, which is
    /// immediately before the router is assembled.
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

    /// Count one Discord membership reconciliation request that ended in `outcome`, in
    /// `tbd_discord_reconcile_outcomes_total`.
    pub fn record_discord_reconcile_outcome(&self, outcome: DiscordReconcileOutcome) {
        self.discord_reconcile_outcomes.record(outcome);
    }

    /// This registry's Discord membership reconciliation outcome counts.
    pub fn discord_reconcile_outcomes(&self) -> &DiscordReconcileOutcomeCounts {
        &self.discord_reconcile_outcomes
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
