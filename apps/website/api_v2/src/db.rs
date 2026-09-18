//! Database layer — Rust port of `internal/db` (pool + migration runner + MV refresh).
//!
//! The migration pipeline is a single frozen `migrations/0001_initial_schema.sql`
//! (the Go GORM-AutoMigrate + raw-SQL schema, proven byte-equal by gate G2). sqlx
//! embeds it at compile time via `migrate!`; future schema changes add new files.
//!
//! T-940.5 — the pool is tuned from [`DbPoolConfig`] (`TBD_DB_POOL_*`). The type lives
//! here, beside its only consumer, rather than in `config.rs`: that file sits 19 lines
//! under the SIZE-3 cap of `cargo xtask verify file-length`, and the allowlist is never
//! extended. `config.rs` re-exports it so `config::DbPoolConfig` resolves.

use std::future::Future;
use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};
use tokio::task::JoinHandle;

use crate::config::ConfigError;

/// Startup connection retry budget (mirrors `db.Open`: 10 attempts, linear backoff).
const CONNECT_ATTEMPTS: u32 = 10;

/// Env var for the background leaderboard MV refresh cadence (seconds).
///
/// Default [`DEFAULT_LEADERBOARD_REFRESH_SECS`] = 15 minutes — short enough that a quiet
/// ingest path cannot leave `leaderboard_totals` stuck at `WITH NO DATA` (or stale) for
/// long, long enough that concurrent refreshes do not thrash the DB under load.
pub const LEADERBOARD_REFRESH_INTERVAL_ENV: &str = "LEADERBOARD_REFRESH_INTERVAL_SECS";

/// Default scheduled refresh interval: 15 minutes.
pub const DEFAULT_LEADERBOARD_REFRESH_SECS: u64 = 15 * 60;

/// Env var: pool ceiling (whole number ≥ 1). Default 25.
pub const DB_POOL_MAX_CONNECTIONS_ENV: &str = "TBD_DB_POOL_MAX_CONNECTIONS";
/// Env var: seconds a connection may sit idle before the pool closes it. Default 300 (5m).
pub const DB_POOL_IDLE_TIMEOUT_ENV: &str = "TBD_DB_POOL_IDLE_TIMEOUT_SECS";
/// Env var: seconds a connection may live before the pool retires it. Default 1800 (30m).
pub const DB_POOL_MAX_LIFETIME_ENV: &str = "TBD_DB_POOL_MAX_LIFETIME_SECS";
/// Env var: seconds a caller waits for a free connection before `PoolTimedOut`. Default 30.
pub const DB_POOL_ACQUIRE_TIMEOUT_ENV: &str = "TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS";

/// sqlx pool tuning — T-940.5. Read from `TBD_DB_POOL_*` by [`DbPoolConfig::from_env`].
///
/// The defaults are the literals this file carried until T-940.5 (`db.Open`'s MaxOpen 25,
/// ConnMaxIdleTime 5m, ConnMaxLifetime 30m; acquire 30s), so an unset environment builds
/// exactly the pool it built before and `cargo xtask db test-it` timing is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbPoolConfig {
    /// Pool ceiling; at least 1. A pool that can never hand out a connection would make
    /// every query wait out `acquire_timeout_secs` and then fail, so `0` is refused at boot.
    pub max_connections: u32,
    /// Idle reap, seconds.
    pub idle_timeout_secs: u64,
    /// Lifetime cap, seconds.
    pub max_lifetime_secs: u64,
    /// Acquire wait, seconds.
    pub acquire_timeout_secs: u64,
}

impl Default for DbPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 25,
            idle_timeout_secs: 5 * 60,
            max_lifetime_secs: 30 * 60,
            acquire_timeout_secs: 30,
        }
    }
}

impl DbPoolConfig {
    /// Read the four `TBD_DB_POOL_*` variables from the process environment.
    ///
    /// Unset or blank = default. Anything else must be a whole number (and ≥ 1 for the
    /// ceiling); a value that is not one is a [`ConfigError::MalformedValue`] naming the
    /// variable, so startup stops there instead of running on a silently-defaulted pool —
    /// the `get_env_int` behaviour this deliberately does not share.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Pure core of [`Self::from_env`]: `lookup` stands in for the process environment, so
    /// the parse rules are unit-tested without mutating it.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        const SECS: &str = "expected a whole number of seconds";
        let d = Self::default();
        Ok(Self {
            max_connections: parse_pool_var(
                DB_POOL_MAX_CONNECTIONS_ENV,
                &lookup,
                d.max_connections,
                "expected a whole number of connections, at least 1",
                |n| *n >= 1,
            )?,
            idle_timeout_secs: parse_pool_var(
                DB_POOL_IDLE_TIMEOUT_ENV,
                &lookup,
                d.idle_timeout_secs,
                SECS,
                |_| true,
            )?,
            max_lifetime_secs: parse_pool_var(
                DB_POOL_MAX_LIFETIME_ENV,
                &lookup,
                d.max_lifetime_secs,
                SECS,
                |_| true,
            )?,
            acquire_timeout_secs: parse_pool_var(
                DB_POOL_ACQUIRE_TIMEOUT_ENV,
                &lookup,
                d.acquire_timeout_secs,
                SECS,
                |_| true,
            )?,
        })
    }
}

/// One `TBD_DB_POOL_*` variable: unset / blank → `fallback`; otherwise it must parse as `T`
/// and satisfy `valid`, or the error carries the variable name and the raw value verbatim.
fn parse_pool_var<T: std::str::FromStr>(
    key: &'static str,
    lookup: &impl Fn(&str) -> Option<String>,
    fallback: T,
    reason: &'static str,
    valid: impl Fn(&T) -> bool,
) -> Result<T, ConfigError> {
    let Some(raw) = lookup(key) else {
        return Ok(fallback);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(fallback);
    }
    match trimmed.parse::<T>() {
        Ok(v) if valid(&v) => Ok(v),
        _ => Err(ConfigError::MalformedValue(key, raw, reason)),
    }
}

/// The [`PgPoolOptions`] a [`DbPoolConfig`] describes — the one place the four knobs meet sqlx.
pub fn pool_options(cfg: &DbPoolConfig) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .idle_timeout(Duration::from_secs(cfg.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(cfg.max_lifetime_secs))
        .acquire_timeout(Duration::from_secs(cfg.acquire_timeout_secs))
}

/// Connect to Postgres with the pool tuned from `TBD_DB_POOL_*` ([`DbPoolConfig::from_env`]),
/// retrying the initial connection with linear backoff (Postgres can briefly refuse
/// connections just after reporting ready).
///
/// A variable that does not parse fails HERE, before any connection attempt, as
/// [`sqlx::Error::Configuration`] wrapping the [`ConfigError`] that names it — so the API
/// binary's `db::connect(&cfg.database_url)?` stops startup with that message, and every
/// caller that opens a pool without loading [`crate::config::Config`] (`import-registry`,
/// the integration suites) gets the same guard.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    connect_with_options(database_url, env_pool_options()?).await
}

/// The [`PgPoolOptions`] [`connect`] builds: [`DbPoolConfig::from_env`] through
/// [`pool_options`], a malformed variable surfaced as [`sqlx::Error::Configuration`].
fn env_pool_options() -> Result<PgPoolOptions, sqlx::Error> {
    DbPoolConfig::from_env()
        .map(|cfg| pool_options(&cfg))
        .map_err(|e| sqlx::Error::Configuration(e.into()))
}

async fn connect_with_options(
    database_url: &str,
    opts: PgPoolOptions,
) -> Result<PgPool, sqlx::Error> {
    let mut last_err: Option<sqlx::Error> = None;
    for attempt in 1..=CONNECT_ATTEMPTS {
        match opts.clone().connect(database_url).await {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(u64::from(attempt) * 250)).await;
            }
        }
    }
    Err(last_err.expect("loop runs at least once"))
}

/// Build a pool that connects lazily (on first use). Used by tests/harnesses that
/// exercise code paths not reaching the DB, without requiring a live server.
///
/// Deliberately NOT env-tuned: a harness pool must neither change shape nor fail to build
/// because the developer's shell happens to export a `TBD_DB_POOL_*` value.
pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(DbPoolConfig::default().max_connections)
        .connect_lazy(database_url)
}

/// Run all pending migrations (embedded from `./migrations` at compile time).
///
/// Mirrors `db.Migrate` — the pre/AutoMigrate/post pipeline is collapsed into the
/// single frozen `0001_initial_schema.sql`.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Refresh the `leaderboard_totals` materialized view. Call after match telemetry
/// ingest (debounced). Falls back to a non-concurrent refresh if the concurrent one
/// fails (e.g. the view has not been populated yet). Mirrors `db.RefreshLeaderboard`.
pub async fn refresh_leaderboard(pool: &PgPool) -> Result<(), sqlx::Error> {
    if sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
        .execute(pool)
        .await
        .is_err()
    {
        sqlx::query("REFRESH MATERIALIZED VIEW leaderboard_totals")
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Resolve the scheduled MV refresh interval from
/// [`LEADERBOARD_REFRESH_INTERVAL_ENV`], falling back to
/// [`DEFAULT_LEADERBOARD_REFRESH_SECS`]. Invalid / zero / negative values use the default.
pub fn leaderboard_refresh_interval() -> Duration {
    leaderboard_refresh_interval_from(
        std::env::var(LEADERBOARD_REFRESH_INTERVAL_ENV)
            .ok()
            .as_deref(),
    )
}

fn leaderboard_refresh_interval_from(raw: Option<&str>) -> Duration {
    match raw {
        Some(s) => match s.trim().parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS),
        },
        None => Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS),
    }
}

/// Spawn the background leaderboard MV refresher: one immediate refresh (so a quiet
/// ingest path cannot leave `leaderboard_totals` at `WITH NO DATA`), then every
/// `interval` until the runtime stops. Failures are logged; the next tick retries.
///
/// Ingest callers of [`refresh_leaderboard`] are unchanged — this is a safety net.
pub fn start_leaderboard_refresh(pool: PgPool, interval: Duration) -> JoinHandle<()> {
    start_leaderboard_refresh_with(
        pool,
        interval,
        |p| async move { refresh_leaderboard(&p).await },
    )
}

/// Testable core of [`start_leaderboard_refresh`]: runs `refresh` immediately, then on
/// each interval tick. The production path wires [`refresh_leaderboard`].
fn start_leaderboard_refresh_with<F, Fut>(
    pool: PgPool,
    interval: Duration,
    refresh: F,
) -> JoinHandle<()>
where
    F: Fn(PgPool) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), sqlx::Error>> + Send + 'static,
{
    tokio::spawn(async move {
        run_refresh(&pool, &refresh).await;
        let mut ticker = tokio::time::interval(interval);
        // `interval` fires immediately on first `tick`; we already refreshed above.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            run_refresh(&pool, &refresh).await;
        }
    })
}

async fn run_refresh<F, Fut>(pool: &PgPool, refresh: &F)
where
    F: Fn(PgPool) -> Fut,
    Fut: Future<Output = Result<(), sqlx::Error>>,
{
    match refresh(pool.clone()).await {
        Ok(()) => tracing::debug!("leaderboard MV refresh ok"),
        Err(e) => tracing::error!(error = %e, "leaderboard MV scheduled refresh failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn refresh_interval_default_when_unset() {
        assert_eq!(
            leaderboard_refresh_interval_from(None),
            Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS)
        );
    }

    #[test]
    fn refresh_interval_parses_positive_secs() {
        assert_eq!(
            leaderboard_refresh_interval_from(Some("30")),
            Duration::from_secs(30)
        );
        assert_eq!(
            leaderboard_refresh_interval_from(Some(" 120 ")),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn refresh_interval_rejects_zero_negative_garbage() {
        let def = Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS);
        assert_eq!(leaderboard_refresh_interval_from(Some("0")), def);
        assert_eq!(leaderboard_refresh_interval_from(Some("-1")), def);
        assert_eq!(leaderboard_refresh_interval_from(Some("nope")), def);
        assert_eq!(leaderboard_refresh_interval_from(Some("")), def);
    }

    /// Perturbation: a stub refresh is invoked on boot and again after each interval tick,
    /// proving the scheduler path (not only ingest) drives refresh.
    ///
    /// Uses a real short interval (no `tokio` `test-util` feature on this crate) and polls
    /// until the expected call counts land.
    #[tokio::test]
    async fn scheduler_invokes_refresh_on_boot_and_interval() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();

        // Lazy pool — never connects; the stub never touches SQL.
        let pool = connect_lazy("postgres://t261-scheduler-test/unused").expect("lazy pool");

        let handle = start_leaderboard_refresh_with(pool, Duration::from_millis(40), move |_p| {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        // Boot refresh runs before the first ticker wait.
        wait_until(
            || calls.load(Ordering::SeqCst) >= 1,
            Duration::from_millis(500),
        )
        .await;
        assert!(calls.load(Ordering::SeqCst) >= 1, "immediate boot refresh");

        // At least one interval tick after boot.
        wait_until(
            || calls.load(Ordering::SeqCst) >= 2,
            Duration::from_millis(500),
        )
        .await;
        assert!(
            calls.load(Ordering::SeqCst) >= 2,
            "interval tick refresh, got {}",
            calls.load(Ordering::SeqCst)
        );

        handle.abort();
        let _ = handle.await;
    }

    // ── T-940.5: DbPoolConfig ────────────────────────────────────────────────────────

    /// Shared by the two tests that mutate `TBD_DB_POOL_*`: `connect` reads all four
    /// variables, so two such tests interleaving would read each other's values.
    static POOL_ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn lookup_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        }
    }

    fn tuned() -> DbPoolConfig {
        DbPoolConfig {
            max_connections: 3,
            idle_timeout_secs: 7,
            max_lifetime_secs: 11,
            acquire_timeout_secs: 13,
        }
    }

    /// Pins the defaults to the literals `connect` carried before T-940.5 — a changed
    /// default is a changed production pool AND a changed `db test-it` profile.
    #[test]
    fn pool_config_defaults_are_the_pre_t940_5_literals() {
        let unset = DbPoolConfig::from_lookup(|_| None).expect("all unset parses");
        assert_eq!(unset, DbPoolConfig::default());
        assert_eq!(
            unset,
            DbPoolConfig {
                max_connections: 25,
                idle_timeout_secs: 300,
                max_lifetime_secs: 1800,
                acquire_timeout_secs: 30,
            }
        );
    }

    #[test]
    fn pool_config_reads_each_override() {
        let cfg = DbPoolConfig::from_lookup(lookup_of(&[
            (DB_POOL_MAX_CONNECTIONS_ENV, "3"),
            (DB_POOL_IDLE_TIMEOUT_ENV, " 7 "),
            (DB_POOL_MAX_LIFETIME_ENV, "11"),
            (DB_POOL_ACQUIRE_TIMEOUT_ENV, "13\n"),
        ]))
        .expect("valid overrides parse");
        assert_eq!(cfg, tuned());
    }

    #[test]
    fn pool_config_blank_means_default() {
        let cfg = DbPoolConfig::from_lookup(lookup_of(&[
            (DB_POOL_MAX_CONNECTIONS_ENV, ""),
            (DB_POOL_ACQUIRE_TIMEOUT_ENV, "  "),
        ]))
        .expect("blank = unset");
        assert_eq!(cfg, DbPoolConfig::default());
    }

    #[test]
    fn pool_config_rejects_each_malformed_value_naming_the_variable() {
        for (key, bad) in [
            (DB_POOL_MAX_CONNECTIONS_ENV, "abc"),
            (DB_POOL_MAX_CONNECTIONS_ENV, "0"),
            (DB_POOL_MAX_CONNECTIONS_ENV, "-1"),
            (DB_POOL_MAX_CONNECTIONS_ENV, "2.5"),
            (DB_POOL_IDLE_TIMEOUT_ENV, "5m"),
            (DB_POOL_MAX_LIFETIME_ENV, "-30"),
            (DB_POOL_ACQUIRE_TIMEOUT_ENV, "thirty"),
        ] {
            let err = DbPoolConfig::from_lookup(lookup_of(&[(key, bad)]))
                .expect_err(&format!("{key}={bad:?} must be refused"));
            assert!(
                matches!(&err, ConfigError::MalformedValue(k, v, _) if *k == key && v == bad),
                "{key}={bad:?}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains(key), "{msg:?} must name {key}");
            assert!(
                msg.contains(&format!("{bad:?}")),
                "{msg:?} must quote {bad:?}"
            );
        }
    }

    #[test]
    fn pool_options_carry_every_knob() {
        let opts = pool_options(&tuned());
        assert_eq!(opts.get_max_connections(), 3);
        assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(7)));
        assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(11)));
        assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(13));
    }

    #[test]
    fn pool_options_default_pins_the_db_open_literals() {
        let opts = pool_options(&DbPoolConfig::default());
        assert_eq!(opts.get_max_connections(), 25);
        assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(5 * 60)));
        assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(30 * 60)));
        assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(30));
    }

    #[tokio::test]
    async fn lazy_pool_keeps_the_default_ceiling() {
        let pool = connect_lazy("postgres://t9405-lazy/unused").expect("lazy pool");
        assert_eq!(pool.options().get_max_connections(), 25);
    }

    /// Acceptance 2 at the `connect` boundary: a non-numeric value refuses to open the pool
    /// — before any connection attempt (nothing listens on :1, and no retry budget is spent)
    /// — with an error naming the variable.
    #[tokio::test]
    async fn connect_refuses_a_non_numeric_pool_var_naming_it() {
        let _env = POOL_ENV.lock().await;
        // SAFETY: see `env_override_yields_a_pool_max_of_3_and_unset_25`.
        unsafe { std::env::set_var(DB_POOL_ACQUIRE_TIMEOUT_ENV, "abc") };
        let res = connect("postgres://t9405:t9405@127.0.0.1:1/t9405_no_such_db").await;
        unsafe { std::env::remove_var(DB_POOL_ACQUIRE_TIMEOUT_ENV) };
        let err = res.expect_err("TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS=abc must refuse to open a pool");
        assert!(matches!(err, sqlx::Error::Configuration(_)), "{err:?}");
        let msg = err.to_string();
        assert!(
            msg.contains(DB_POOL_ACQUIRE_TIMEOUT_ENV) && msg.contains("\"abc\""),
            "{msg:?} must name the variable and quote the value"
        );
    }

    /// T-940.5 acceptance — `TBD_DB_POOL_MAX_CONNECTIONS=3` yields a pool whose max is 3, and
    /// unset yields 25. Builds the very [`PgPoolOptions`] [`connect`] would open with
    /// ([`env_pool_options`]) into a lazy pool, so the proof needs no database and reads no
    /// `TEST_DATABASE_URL` (the T-542/T-558 pin forbids that outside `tests/common`); the
    /// eager `.connect()` in `connect_with_options` is unchanged from before T-940.5.
    #[tokio::test]
    async fn env_override_yields_a_pool_max_of_3_and_unset_25() {
        let _env = POOL_ENV.lock().await;
        // SAFETY: the two tests that touch `TBD_DB_POOL_*` serialise on `POOL_ENV`; both writes
        // happen on this thread, every reader is Rust `std::env` (internally locked), and
        // nothing in the process calls `getenv` from C during the window.
        unsafe { std::env::set_var(DB_POOL_MAX_CONNECTIONS_ENV, "3") };
        let overridden = env_pool_options();
        unsafe { std::env::remove_var(DB_POOL_MAX_CONNECTIONS_ENV) };
        let unset = env_pool_options();
        let url = "postgres://t9405-env/unused";
        let overridden = overridden
            .expect("TBD_DB_POOL_MAX_CONNECTIONS=3 parses")
            .connect_lazy(url)
            .expect("lazy pool");
        let unset = unset
            .expect("unset parses")
            .connect_lazy(url)
            .expect("lazy pool");
        assert_eq!(
            overridden.options().get_max_connections(),
            3,
            "TBD_DB_POOL_MAX_CONNECTIONS=3 must yield a pool max of 3"
        );
        assert_eq!(
            unset.options().get_max_connections(),
            25,
            "unset TBD_DB_POOL_MAX_CONNECTIONS must yield the default 25"
        );
    }

    async fn wait_until(mut pred: impl FnMut() -> bool, budget: Duration) {
        let start = tokio::time::Instant::now();
        while !pred() {
            assert!(
                start.elapsed() < budget,
                "timed out waiting for scheduler refresh"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}
