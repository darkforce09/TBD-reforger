//! sqlx pool tuning read from the four `TBD_DB_POOL_*` environment variables.
//!
//! The defaults build the pool the API runs with when nothing is set, so an unset environment
//! and a deliberately-tuned one differ only where the operator said so. A value that is set but
//! does not parse is a boot failure naming the variable — never a silent fallback to the
//! default, which would leave an operator believing a knob they typed is in effect.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

use crate::core::configuration::ConfigError;

/// Env var: pool ceiling (whole number ≥ 1). Default 25.
pub const DB_POOL_MAX_CONNECTIONS_ENV: &str = "TBD_DB_POOL_MAX_CONNECTIONS";
/// Env var: seconds a connection may sit idle before the pool closes it. Default 300 (5m).
pub const DB_POOL_IDLE_TIMEOUT_ENV: &str = "TBD_DB_POOL_IDLE_TIMEOUT_SECS";
/// Env var: seconds a connection may live before the pool retires it. Default 1800 (30m).
pub const DB_POOL_MAX_LIFETIME_ENV: &str = "TBD_DB_POOL_MAX_LIFETIME_SECS";
/// Env var: seconds a caller waits for a free connection before `PoolTimedOut`. Default 30.
pub const DB_POOL_ACQUIRE_TIMEOUT_ENV: &str = "TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS";

/// sqlx pool tuning. Read from `TBD_DB_POOL_*` by [`DbPoolConfig::from_env`].
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
    /// variable, so startup stops there instead of running on a silently-defaulted pool.
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

#[cfg(test)]
#[path = "tests/connection_pool.rs"]
mod tests;
