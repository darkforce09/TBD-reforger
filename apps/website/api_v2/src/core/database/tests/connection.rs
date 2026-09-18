use super::*;

/// Shared by the two tests that mutate `TBD_DB_POOL_*`: `connect` reads all four
/// variables, so two such tests interleaving would read each other's values.
static POOL_ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn lazy_pool_keeps_the_default_ceiling() {
    let pool = connect_lazy("postgres://lazy-pool/unused").expect("lazy pool");
    assert_eq!(pool.options().get_max_connections(), 25);
}

/// A non-numeric pool variable refuses to open the pool — before any connection attempt
/// (nothing listens on :1, and no retry budget is spent) — with an error naming the variable.
#[tokio::test]
async fn connect_refuses_a_non_numeric_pool_var_naming_it() {
    let _env = POOL_ENV.lock().await;
    // SAFETY: see `env_override_yields_a_pool_max_of_3_and_unset_25`.
    unsafe { std::env::set_var(connection_pool::DB_POOL_ACQUIRE_TIMEOUT_ENV, "abc") };
    let res = connect("postgres://pool:pool@127.0.0.1:1/no_such_db").await;
    unsafe { std::env::remove_var(connection_pool::DB_POOL_ACQUIRE_TIMEOUT_ENV) };
    let err = res.expect_err("TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS=abc must refuse to open a pool");
    assert!(matches!(err, sqlx::Error::Configuration(_)), "{err:?}");
    let msg = err.to_string();
    assert!(
        msg.contains(connection_pool::DB_POOL_ACQUIRE_TIMEOUT_ENV) && msg.contains("\"abc\""),
        "{msg:?} must name the variable and quote the value"
    );
}

/// `TBD_DB_POOL_MAX_CONNECTIONS=3` yields a pool whose max is 3, and unset yields 25. Builds
/// the very [`PgPoolOptions`] [`connect`] would open with ([`env_pool_options`]) into a lazy
/// pool, so the proof needs no database and reads no `TEST_DATABASE_URL` (which is confined to
/// `tests/common`); the eager `.connect()` in `connect_with_options` is unaffected.
#[tokio::test]
async fn env_override_yields_a_pool_max_of_3_and_unset_25() {
    let _env = POOL_ENV.lock().await;
    // SAFETY: the two tests that touch `TBD_DB_POOL_*` serialise on `POOL_ENV`; both writes
    // happen on this thread, every reader is Rust `std::env` (internally locked), and
    // nothing in the process calls `getenv` from C during the window.
    unsafe { std::env::set_var(connection_pool::DB_POOL_MAX_CONNECTIONS_ENV, "3") };
    let overridden = env_pool_options();
    unsafe { std::env::remove_var(connection_pool::DB_POOL_MAX_CONNECTIONS_ENV) };
    let unset = env_pool_options();
    let url = "postgres://pool-env/unused";
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
