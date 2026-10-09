//! A held PostgreSQL row lock, the bounded wait until requests queue behind it, and the backend of
//! a transaction held open at a paused failpoint.
//!
//! **Role:** orders contenders deterministically without sleeps: a case takes a row lock the
//! production transactions also take (or pauses a production transaction at a failpoint and finds
//! its backend with [`paused_transaction_backend`]), starts its requests, waits until PostgreSQL
//! reports them blocked, then releases the lock so they proceed in lock-queue order.
//! **Position:** used by the `controlled_races*` and `failure_injection*` suites; the waiting
//! query follows the lock-barrier pattern of `tests/reservation_guard_support` and reads
//! `pg_stat_activity` and `pg_blocking_pids` of the suite's own database.
//! **Signals & state:** a [`RowLockHolder`] owns one open transaction on its own pooled
//! connection until it is released or dropped (a drop rolls back).
//! **Invariants:** acquiring locks at least one row or panics; every wait is bounded by
//! [`BLOCKED_WAIT_BOUND`] and panics when the expected waiters or the paused transaction never
//! appear; waiters queued behind another waiter count, not only direct waiters of the holder.

use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction};

/// How long a case waits for its requests to queue behind a held lock before it fails.
pub(crate) const BLOCKED_WAIT_BOUND: Duration = Duration::from_secs(10);

/// How often the wait for the paused transaction reads the backends again.
const PAUSED_BACKEND_POLL: Duration = Duration::from_millis(10);

/// One open transaction holding the row locks its statement took.
///
/// ```ignore
/// let holder = RowLockHolder::acquire(&pool, "SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE", event).await;
/// let request = tokio::spawn(claim_seat());
/// holder.wait_for_blocked(&pool, 1).await; holder.release().await;
/// ```
pub(crate) struct RowLockHolder {
    transaction: Transaction<'static, Postgres>,
    backend_pid: i32,
}

impl RowLockHolder {
    /// Opens a transaction, runs `lock_statement` (a `SELECT … FOR UPDATE` or `FOR NO KEY
    /// UPDATE` with one `$1` placeholder bound to `key`) and keeps its locks.
    ///
    /// # Panics
    ///
    /// When the statement fails or locks no row.
    pub(crate) async fn acquire<K>(pool: &PgPool, lock_statement: &'static str, key: K) -> Self
    where
        K: for<'q> sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres> + Send + 'static,
    {
        let mut transaction = pool
            .begin()
            .await
            .expect("begin the lock-holding transaction");
        let locked = sqlx::query(lock_statement)
            .bind(key)
            .fetch_all(&mut *transaction)
            .await
            .expect("the lock statement runs");
        assert!(
            !locked.is_empty(),
            "the lock statement must lock at least one row: {lock_statement}"
        );
        let backend_pid = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *transaction)
            .await
            .expect("read the holder's backend pid");
        Self {
            transaction,
            backend_pid,
        }
    }

    /// The PostgreSQL backend that holds the locks.
    pub(crate) fn backend_pid(&self) -> i32 {
        self.backend_pid
    }

    /// Waits until at least `minimum` backends queue behind this holder; see [`wait_for_blocked`].
    pub(crate) async fn wait_for_blocked(&self, pool: &PgPool, minimum: i64) {
        wait_for_blocked(pool, self.backend_pid, minimum).await;
    }

    /// Rolls the holding transaction back, which frees its locks; the queued requests proceed.
    pub(crate) async fn release(self) {
        self.transaction
            .rollback()
            .await
            .expect("roll the lock-holding transaction back");
    }
}

/// Waits until at least `minimum` backends of the current database are blocked behind `owner`,
/// directly or behind another waiter.
///
/// # Panics
///
/// When they do not queue within [`BLOCKED_WAIT_BOUND`].
pub(crate) async fn wait_for_blocked(pool: &PgPool, owner: i32, minimum: i64) {
    tokio::time::timeout(BLOCKED_WAIT_BOUND, async {
        loop {
            // Waiters queued behind another waiter count, not only direct waiters of the owner.
            let count: i64 = sqlx::query_scalar(
                "WITH RECURSIVE blocked(pid) AS (
                 SELECT pid FROM pg_stat_activity
                 WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))
                 UNION SELECT a.pid FROM pg_stat_activity a JOIN blocked b ON b.pid = ANY(pg_blocking_pids(a.pid))
                 WHERE a.datname = current_database()) SELECT count(*) FROM blocked",
            )
            .bind(owner)
            .fetch_one(pool)
            .await
            .expect("read the blocked backends");
            if count >= minimum {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!("{minimum} request(s) must queue behind backend {owner} within {BLOCKED_WAIT_BOUND:?}")
    });
}

/// The backend of the transaction held open at a paused failpoint: the one backend of the current
/// database idle inside a transaction. The caller holds the failpoint suite lock and has no
/// transaction of its own open, so no other backend of the database can be in that state.
///
/// # Panics
///
/// When no such backend appears within [`BLOCKED_WAIT_BOUND`], or when two do.
pub(crate) async fn paused_transaction_backend(pool: &PgPool) -> i32 {
    tokio::time::timeout(BLOCKED_WAIT_BOUND, async {
        loop {
            let backends: Vec<i32> = sqlx::query_scalar(
                "SELECT pid FROM pg_stat_activity
                 WHERE datname = current_database() AND state = 'idle in transaction'",
            )
            .fetch_all(pool)
            .await
            .expect("read the backends idle inside a transaction");
            match backends.as_slice() {
                [backend] => return *backend,
                [] => tokio::time::sleep(PAUSED_BACKEND_POLL).await,
                _ => panic!("one transaction must wait at the paused point, found {backends:?}"),
            }
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!("no transaction waited at the paused point within {BLOCKED_WAIT_BOUND:?}")
    })
}
