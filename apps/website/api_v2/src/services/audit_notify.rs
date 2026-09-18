//! T-940.6 — Postgres `LISTEN audit_log`: the push behind the admin audit stream.
//!
//! Migration 0025 raises `pg_notify('audit_log', <id>)` from an AFTER INSERT trigger on
//! `audit_logs`, so every row — [`super::write_audit`]'s and the trigger-written event create /
//! mission soft-delete / slot kick rows alike — is announced the moment it commits. This module
//! holds the process's one `PgListener` per pool and fans those announcements out to every
//! connected `GET /admin/audit-logs/stream` as [`AuditSignal`]s over a broadcast channel.
//!
//! Shape:
//! - **One listener per pool, not per client.** [`AuditNotify::for_pool`] registers the pool
//!   (keyed by its connect-options allocation, so every clone of a `PgPool` maps to the same
//!   entry) and spawns the pump task on first use. A subscriber costs a broadcast receiver, not a
//!   database connection; the pump pins exactly one pooled connection while it is up.
//! - **Reconnect with backoff, explicit up/down.** sqlx's `PgListener` can redial on its own, but
//!   silently — and a silent gap is exactly when the stream must poll instead. So the pump sets
//!   `eager_reconnect(false)`: a lost connection surfaces as `Ok(None)`, the pump flips
//!   [`AuditNotify::is_listening`] to `false`, broadcasts [`AuditSignal::Down`], and redials with
//!   [`BACKOFF_INITIAL`] → [`BACKOFF_MAX`] exponential backoff. A successful `LISTEN` flips it back
//!   and broadcasts [`AuditSignal::Resync`]: notifications raised during the gap are gone for good,
//!   so the stream catches up from the table.
//! - **Runtime-safe respawn.** The pump is a plain spawned task. If the runtime that ran it goes
//!   away (a `#[tokio::test]` ending), its guard drops, the registry entry reads as orphaned, and
//!   the next [`AuditNotify::for_pool`] respawns it on the caller's runtime.
//!
//! `handlers::audit::audit_row_stream` is the consumer: it fetches `id > last_id` on `Row` and
//! `Resync`, and lets its 2 s ticker reach the database only while `is_listening()` is false.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::{PgListener, PgNotification};
use tokio::sync::broadcast;

/// The channel `0025_audit_notify.sql` raises on every `audit_logs` insert.
pub const AUDIT_CHANNEL: &str = "audit_log";

/// First redial delay after a lost connection; doubles up to [`BACKOFF_MAX`].
pub const BACKOFF_INITIAL: Duration = Duration::from_millis(250);
/// Redial delay ceiling while the database stays unreachable.
pub const BACKOFF_MAX: Duration = Duration::from_secs(5);

/// Broadcast depth per pool. A subscriber that falls this far behind sees `Lagged`, which the
/// stream treats as "fetch everything newer than what you have" — coalesced, not lost.
const BROADCAST_CAPACITY: usize = 256;

/// What a subscriber receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSignal {
    /// An `audit_logs` row with this id committed.
    Row(i64),
    /// The listener (re)connected: rows may have landed unannounced — catch up from the table.
    Resync,
    /// The listener lost its connection; poll until `Resync`.
    Down,
}

/// The redial delay after `current`: doubled, capped at [`BACKOFF_MAX`].
pub fn next_backoff(current: Duration) -> Duration {
    current.saturating_mul(2).min(BACKOFF_MAX)
}

/// The signal one notification carries. A foreign channel is nobody's business (`None`); an
/// `audit_log` payload that is not an id still means "something landed", so it resyncs.
pub fn signal_for(channel: &str, payload: &str) -> Option<AuditSignal> {
    if channel != AUDIT_CHANNEL {
        return None;
    }
    Some(
        payload
            .trim()
            .parse::<i64>()
            .map_or(AuditSignal::Resync, AuditSignal::Row),
    )
}

struct Shared {
    tx: broadcast::Sender<AuditSignal>,
    /// `LISTEN audit_log` is active on a live connection.
    listening: AtomicBool,
    /// A pump task currently owns this entry (cleared by [`PumpGuard`] when that task is dropped).
    pump_alive: AtomicBool,
    /// Backend pid of the listening connection, 0 while down — so a test or an operator can
    /// `pg_terminate_backend` exactly the listener.
    backend_pid: AtomicU32,
    /// `user@host:port/db`, for log lines.
    label: String,
}

impl Shared {
    fn new(label: String) -> Self {
        Self {
            tx: broadcast::channel(BROADCAST_CAPACITY).0,
            listening: AtomicBool::new(false),
            pump_alive: AtomicBool::new(false),
            backend_pid: AtomicU32::new(0),
            label,
        }
    }
}

/// Handle on the per-pool listener. Cheap to clone; every clone shares the one pump.
#[derive(Clone)]
pub struct AuditNotify {
    shared: Arc<Shared>,
}

fn registry() -> &'static Mutex<HashMap<usize, Arc<Shared>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Arc<Shared>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Identity of a pool across its clones: the `Arc` behind `connect_options()` is allocated once
/// per `PgPool::connect*` and shared by every clone.
fn pool_key(pool: &PgPool) -> usize {
    Arc::as_ptr(&pool.connect_options()) as usize
}

fn pool_label(pool: &PgPool) -> String {
    let o = pool.connect_options();
    format!(
        "{}@{}:{}/{}",
        o.get_username(),
        o.get_host(),
        o.get_port(),
        o.get_database().unwrap_or("")
    )
}

impl AuditNotify {
    /// The listener for `pool`, spawning its pump on first use — or again, if the runtime that
    /// ran the previous pump is gone. Call from inside a Tokio runtime; without one the handle
    /// comes back with the listener down, so the caller polls.
    pub fn for_pool(pool: &PgPool) -> Self {
        let shared = {
            let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
            Arc::clone(
                reg.entry(pool_key(pool))
                    .or_insert_with(|| Arc::new(Shared::new(pool_label(pool)))),
            )
        };
        if !shared.pump_alive.swap(true, Ordering::AcqRel) {
            match tokio::runtime::Handle::try_current() {
                Ok(rt) => {
                    rt.spawn(pump(pool.clone(), Arc::clone(&shared)));
                }
                Err(_) => {
                    shared.pump_alive.store(false, Ordering::Release);
                    tracing::warn!(
                        db = %shared.label,
                        "audit listener not started: no tokio runtime; the stream will poll"
                    );
                }
            }
        }
        Self { shared }
    }

    /// A receiver for every signal from now on.
    pub fn subscribe(&self) -> broadcast::Receiver<AuditSignal> {
        self.shared.tx.subscribe()
    }

    /// `true` while `LISTEN audit_log` is active on a live connection.
    pub fn is_listening(&self) -> bool {
        self.shared.listening.load(Ordering::Acquire)
    }

    /// Backend pid of the listening connection, `None` while down.
    pub fn backend_pid(&self) -> Option<u32> {
        match self.shared.backend_pid.load(Ordering::Acquire) {
            0 => None,
            pid => Some(pid),
        }
    }
}

/// Resets the entry when the pump's future is dropped — a runtime shutdown, not a return path —
/// so subscribers fall back to polling and the next `for_pool` can respawn.
struct PumpGuard(Arc<Shared>);

impl Drop for PumpGuard {
    fn drop(&mut self) {
        self.0.listening.store(false, Ordering::Release);
        self.0.backend_pid.store(0, Ordering::Release);
        self.0.pump_alive.store(false, Ordering::Release);
        let _ = self.0.tx.send(AuditSignal::Down);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Lost {
    Connection,
    PoolClosed,
}

async fn pump(pool: PgPool, shared: Arc<Shared>) {
    let _guard = PumpGuard(Arc::clone(&shared));
    let mut backoff = BACKOFF_INITIAL;
    loop {
        if pool.is_closed() {
            return;
        }
        match connect(&pool).await {
            Ok((mut listener, pid)) => {
                backoff = BACKOFF_INITIAL;
                shared.backend_pid.store(pid, Ordering::Release);
                shared.listening.store(true, Ordering::Release);
                let _ = shared.tx.send(AuditSignal::Resync);
                tracing::info!(db = %shared.label, pid, "audit listener up");

                let lost = receive_until_lost(&mut listener, &shared).await;

                shared.listening.store(false, Ordering::Release);
                shared.backend_pid.store(0, Ordering::Release);
                // Give the pool slot back BEFORE announcing `Down`, so nothing reacting to `Down`
                // races this task for the slot it just released.
                drop(listener);
                let _ = shared.tx.send(AuditSignal::Down);
                if lost == Lost::PoolClosed {
                    return;
                }
                tracing::warn!(
                    db = %shared.label,
                    redial_in = ?backoff,
                    "audit listener lost its connection"
                );
            }
            Err(e) => {
                if pool.is_closed() {
                    return;
                }
                tracing::warn!(
                    db = %shared.label,
                    error = %e,
                    redial_in = ?backoff,
                    "audit listener could not connect"
                );
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = next_backoff(backoff);
    }
}

/// One pooled connection, `LISTEN audit_log` issued, and its backend pid.
async fn connect(pool: &PgPool) -> Result<(PgListener, u32), sqlx::Error> {
    let mut listener = PgListener::connect_with(pool).await?;
    // Reconnection is this module's job: an explicit `Down` is what lets the stream poll.
    listener.eager_reconnect(false);
    listener.listen(AUDIT_CHANNEL).await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut listener)
        .await?;
    Ok((listener, u32::try_from(pid).unwrap_or(0)))
}

/// Forward notifications until the connection is gone. `Ok(None)` is sqlx's "connection lost"
/// (not redialed, see `eager_reconnect(false)`); any other error is treated the same way.
async fn receive_until_lost(listener: &mut PgListener, shared: &Shared) -> Lost {
    loop {
        match listener.try_recv().await {
            Ok(Some(n)) => forward(shared, &n),
            Ok(None) => return Lost::Connection,
            Err(sqlx::Error::PoolClosed) => return Lost::PoolClosed,
            Err(e) => {
                tracing::warn!(db = %shared.label, error = %e, "audit listener receive failed");
                return Lost::Connection;
            }
        }
    }
}

fn forward(shared: &Shared, n: &PgNotification) {
    if let Some(sig) = signal_for(n.channel(), n.payload()) {
        // `Err` means no subscriber right now — nothing to deliver to.
        let _ = shared.tx.send(sig);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RED: cap the doubling at the wrong ceiling, or stop doubling — the chain below diverges.
    #[test]
    fn backoff_doubles_and_caps() {
        let mut d = BACKOFF_INITIAL;
        let mut chain = Vec::new();
        for _ in 0..8 {
            chain.push(d.as_millis());
            d = next_backoff(d);
        }
        assert_eq!(chain, [250, 500, 1000, 2000, 4000, 5000, 5000, 5000]);
        assert_eq!(next_backoff(BACKOFF_MAX), BACKOFF_MAX);
    }

    /// RED: forward every channel, or map a non-numeric payload to `None` — the asserts fire.
    #[test]
    fn signal_for_reads_the_row_id_and_ignores_other_channels() {
        assert_eq!(signal_for(AUDIT_CHANNEL, "42"), Some(AuditSignal::Row(42)));
        assert_eq!(signal_for(AUDIT_CHANNEL, " 7 "), Some(AuditSignal::Row(7)));
        assert_eq!(
            signal_for(AUDIT_CHANNEL, "not-an-id"),
            Some(AuditSignal::Resync)
        );
        assert_eq!(signal_for(AUDIT_CHANNEL, ""), Some(AuditSignal::Resync));
        assert_eq!(signal_for("server_status", "42"), None);
    }
}
