//! Per-binary integration database: target guard, name derivation, and provisioning.
//!
//! Every DB-backed suite calls [`require_test_database_url`] instead of reading
//! `TEST_DATABASE_URL` itself. That single entry point is what makes two guarantees hold for
//! all of them at once: the URL can only ever point at an allow-listed throwaway database,
//! and each test binary gets a private one, provisioned once, so a binary's verdict cannot
//! depend on rows a sibling binary left behind. The tests inside one binary share that
//! database and isolate themselves through rows and ids they mint for themselves.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use sqlx::{AssertSqlSafe, Connection, PgConnection};
use url::Url;

use super::http::{DEV_LOGIN_ARMA_ID, DEV_LOGIN_USER};

// ───────────────────────────── target guard ─────────────────────────────

/// Extract the PostgreSQL database name from a connection URL's path.
///
/// `postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable` → `Some("rust_it")`.
/// Empty path, unparseable URL, or a multi-segment path → `None`.
pub(crate) fn database_name_from_url(database_url: &str) -> Option<String> {
    let parsed = Url::parse(database_url).ok()?;
    let name = parsed.path().trim_start_matches('/');
    if name.is_empty() || name.contains('/') {
        return None;
    }
    // Percent-decoding is unnecessary for our ASCII test DB names; reject weirdness.
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Some(name.to_string())
}

/// Whether `name` is a dedicated integration / gate / probe database — never the live
/// `tbd_reforger` dev DB.
///
/// Allow-list (the xtask database commands + operator scratch databases):
/// - `rust_it` — `cargo xtask db test-it` (dropped and recreated per run)
/// - `tbd_gate*` — `cargo xtask db selftest` and the deploy database drills (`tbd_gate_it`,
///   `tbd_gate_migrate`, …)
/// - `*_cold` — operator `TBD_GATE_DB` cold databases (`tbd_operator_cold`, …)
/// - `*_it` / `*_probe` — agent throwaways that already follow the naming convention
///
/// Anything else (notably `tbd_reforger`) is refused so an exported
/// `TEST_DATABASE_URL=…/tbd_reforger` cannot wipe the live database.
pub(crate) fn is_safe_test_database_name(name: &str) -> bool {
    if name.is_empty() || name == "tbd_reforger" {
        return false;
    }
    name == "rust_it"
        || name.starts_with("tbd_gate")
        || name.ends_with("_cold")
        || name.ends_with("_it")
        || name.ends_with("_probe")
}

/// Fail loud if `database_url` does not point at a safe test database name.
///
/// Call this immediately after reading `TEST_DATABASE_URL` (and before connect /
/// migrate / any DELETE). The resolver rejects an absent URL before this guard is reached.
pub(crate) fn assert_test_database_url(database_url: &str) {
    let name = database_name_from_url(database_url).unwrap_or_else(|| {
        panic!(
            "\n\
             ───────────────────────────────────────────────────────────────────────\n\
             TEST_DATABASE_URL is set but its database name could not be parsed.\n\
             \n  \
             url: <redacted>\n\
             \n  \
             Expected a postgres URL whose path is a single ASCII name, e.g.\n  \
             postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable\n\
             ───────────────────────────────────────────────────────────────────────"
        )
    });
    if !is_safe_test_database_name(&name) {
        panic!(
            "\n\
             ───────────────────────────────────────────────────────────────────────\n\
             TEST_DATABASE_URL refuses to target database `{name}`.\n\
             \n  \
             url: <redacted>\n\
             \n  \
             Allowed names: rust_it, tbd_gate*, *_cold, *_it, *_probe.\n  \
             The live dev database `tbd_reforger` is never allowed — pointing the\n  \
             integration suite at it would wipe production-like rows.\n  \
             Fix: `cargo xtask db test-it` (creates rust_it), or export a URL whose path\n  \
             matches the allow-list (tbd_gate* / *_cold / *_it / *_probe).\n\
             ───────────────────────────────────────────────────────────────────────"
        );
    }
}

// ─────────────────────── one database per test BINARY ───────────────────────

/// The test binary this copy of `common` was compiled into.
///
/// Cargo compiles one crate per `tests/<binary>/main.rs`, and `CARGO_CRATE_NAME` is set per
/// compilation unit — so this expands to `missions` inside `tests/missions/main.rs`'s binary
/// and to `smoke` inside `tests/smoke/main.rs`'s. It is a compile-time `env!`, so a Cargo that
/// stopped setting it is a build error here rather than a silent fallback to one shared name,
/// which would let two binaries drop each other's database.
const SUITE: &str = env!("CARGO_CRATE_NAME");

/// Resolved once per test binary. Missing configuration panics; initialized values are Some.
static PER_BINARY_URL: OnceLock<Option<String>> = OnceLock::new();

/// Derive this binary's private database name from the operator's base name.
///
/// `("rust_it", "missions")` → `"rust_it_missions_it"`.
///
/// The `_it` suffix is not decoration: it is what keeps every generated name inside the
/// allow-list ([`is_safe_test_database_name`]) no matter what the base was — `rust_it`,
/// `tbd_gate_migrate` and `tbd_x_cold` all derive to a `*_it` name. The suffix is re-checked at
/// runtime in [`resolve_and_provision`]; this function is not trusted to have got it right.
///
/// Postgres truncates identifiers at 63 bytes, and a truncated name is a name two binaries
/// can share — which is exactly the defect. Over-long names therefore fold their tail into a
/// hash rather than losing it.
pub(crate) fn per_binary_database_name(base: &str, suite: &str) -> String {
    let sanitised: String = suite
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let name = format!("{base}_{sanitised}_it");
    if name.len() <= 63 {
        return name;
    }
    // FNV-1a over the full untruncated name: stable across runs and processes (unlike
    // DefaultHasher, which is randomly seeded per process and would hand the same binary a
    // different database on every run).
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // 63 = keep + 1 ('_') + 16 (hex) + 3 ("_it")
    format!("{}_{hash:016x}_it", &name[..43])
}

/// Swap the database name in a Postgres URL, preserving user/host/port/query.
pub(crate) fn with_database_name(url: &str, database: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    parsed.set_path(database);
    Some(parsed.into())
}

/// Read `TEST_DATABASE_URL` and hand back **this binary's own** database URL.
///
/// Always returns Some after provisioning. Panics on missing configuration or an unsafe target.
/// The Option return shape preserves existing fixture callers without allowing skipped tests.
///
/// # Why this does not return what the operator exported
///
/// If every DB-backed binary connected to the single `TEST_DATABASE_URL`, a suite's verdict
/// would depend on what its siblings left in `users`, `missions`, `user_factions`, … The
/// symptom is a gate whose result is not reproducible even against a fresh database, because
/// the residue is made **within** one run.
///
/// The cure is one database per binary, created here on first call and named
/// `<base>_<suite>_it` (see [`per_binary_database_name`]). It is dropped and recreated on
/// every run, so a binary's verdict cannot depend on a previous run either, and the
/// allow-list is asserted on the **derived** name as well as the operator's.
///
/// It does not serialise anything: tests still run in parallel inside a binary, at full
/// speed, each on rows and ids it minted itself.
///
/// # Known limit: concurrent `cargo test` processes still race
///
/// Two concurrent `cargo test` invocations against the **same** operator base race on
/// `<base>_<suite>_it`: both binaries run [`provision`]'s `DROP DATABASE … WITH (FORCE)` /
/// `CREATE DATABASE` for the same derived name. `cargo xtask db test-it` runs the binaries one
/// after another, so that path is covered; cross-process overlap outside it would need
/// PID-suffixed names or a cross-process provision lock.
pub(crate) fn require_test_database_url() -> Option<String> {
    PER_BINARY_URL
        .get_or_init(|| Some(resolve_and_provision(SUITE)))
        .clone()
}

/// The isolated databases provisioned so far, by scope.
static ISOLATED_URLS: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

/// A database of this binary's own beside the shared one, `<base>_<binary>_<scope>_it`, for a
/// module whose assertions read or change whole tables or a shared account (a seeded golden
/// reproduced row for row, a board over every player, a replay that revokes every session of the
/// dev-login account) and so cannot share rows with the binary's other tests.
///
/// Dropped, recreated and migrated on the first call for `scope`; later calls return the same
/// URL.
pub(crate) fn require_isolated_test_database_url(scope: &str) -> String {
    let mut urls = ISOLATED_URLS.lock().unwrap_or_else(PoisonError::into_inner);
    urls.entry(scope.to_string())
        .or_insert_with(|| resolve_and_provision(&format!("{SUITE}_{scope}")))
        .clone()
}

/// Derive the database name for `suite`, guard it, create the database, migrate it, and prime
/// the shared `dev-login` row.
fn resolve_and_provision(suite: &str) -> String {
    let base_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        panic!("TEST_DATABASE_URL is required for database tests, including CI and verification gates; run cargo xtask db test-it. A test without its database cannot pass.")
    });
    // The operator's own name is checked first and unchanged — a URL pointing at the live
    // database must still panic here, before anything below can create or drop anything.
    assert_test_database_url(&base_url);
    let base_name =
        database_name_from_url(&base_url).expect("assert_test_database_url accepted the URL");

    let derived_name = per_binary_database_name(&base_name, suite);
    assert!(
        derived_name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "derived database name `{derived_name}` is not a bare identifier — it is \
         interpolated into DDL below and must never need quoting or escaping"
    );
    let derived_url = with_database_name(&base_url, &derived_name)
        .unwrap_or_else(|| panic!("cannot rewrite test database name"));
    // The guard applies to what we actually connect to, not only to what was exported.
    // If a future base name derives to something the allow-list refuses, that is a hard
    // stop here rather than a database created outside the list.
    assert_test_database_url(&derived_url);

    provision(&base_url, &derived_name, &derived_url);
    derived_url
}

/// Run [`provision_async`] on its own thread + runtime.
///
/// `require_test_database_url` is called from inside `#[tokio::test]` bodies, so it cannot
/// `block_on` here — a nested `block_on` panics. A dedicated thread owning its own
/// current-thread runtime keeps the caller's signature synchronous, which is what lets
/// `OnceLock` do the once-per-process serialisation with no changes at the call sites.
fn provision(base_url: &str, derived_name: &str, derived_url: &str) {
    let label = derived_name.to_string();
    let (base_url, derived_name, derived_url) = (
        base_url.to_string(),
        derived_name.to_string(),
        derived_url.to_string(),
    );
    let handle = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build provisioning runtime")
            .block_on(provision_async(&base_url, &derived_name, &derived_url));
    });
    if let Err(payload) = handle.join() {
        let why = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic payload>");
        panic!("provisioning `{label}` for the `{SUITE}` test binary failed: {why}");
    }
}

async fn provision_async(base_url: &str, derived_name: &str, derived_url: &str) {
    // Maintenance session on the operator's own (allow-listed) database — never `postgres`,
    // so no connection this harness opens is outside the allow-list. DROP/CREATE DATABASE
    // cannot run against the database being dropped, which is the only reason a second
    // database is involved at all.
    let mut admin = PgConnection::connect(base_url).await.unwrap_or_else(|e| {
        panic!(
            "connect to base database to create the `{SUITE}` test binary's database: {e}\n  \
             The base database must exist — `cargo xtask db test-it` creates rust_it."
        )
    });
    // WITH (FORCE) so a leaked connection from a killed run cannot pin the name. The target
    // is `<base>_<suite>_it`, a name nothing but this binary ever uses.
    // `raw_sql`, not `query`: DDL must go over the SIMPLE protocol. sqlx's own guidance says
    // so, and `CREATE DATABASE` is one of the statements Postgres refuses inside the implicit
    // transaction the extended protocol opens. `AssertSqlSafe` is load-bearing rather than
    // decorative — `derived_name` was asserted to be a bare `[a-z0-9_]` identifier above, which
    // is what makes interpolating it here safe.
    let drop_sql = format!("DROP DATABASE IF EXISTS {derived_name} WITH (FORCE)");
    let create_sql = format!("CREATE DATABASE {derived_name}");
    sqlx::raw_sql(AssertSqlSafe(drop_sql))
        .execute(&mut admin)
        .await
        .unwrap_or_else(|e| panic!("DROP DATABASE {derived_name}: {e}"));
    sqlx::raw_sql(AssertSqlSafe(create_sql))
        .execute(&mut admin)
        .await
        .unwrap_or_else(|e| panic!("CREATE DATABASE {derived_name}: {e}"));
    admin
        .close()
        .await
        .unwrap_or_else(|e| panic!("close maintenance connection: {e}"));

    let pool = api_database::connect(derived_url)
        .await
        .unwrap_or_else(|e| panic!("connect to derived test database: {e}"));
    api_database::migrate(&pool)
        .await
        .unwrap_or_else(|e| panic!("migrate `{derived_name}`: {e}"));

    // Prime the shared dev-login row. Read this before deleting it.
    //
    // The handler itself is race-free: it inserts a NULL `arma_id` and stamps it with
    // `COALESCE(arma_id, …)` on first create, so concurrent first-time calls no longer
    // collide on `idx_users_arma_id`. The prime stays because integration suites want the
    // production row shape (a linked `arma_id`) in place before any test runs, and because
    // it remains defence in depth if a future edit reintroduces the racing INSERT shape.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
         arma_character, role, is_banned, ban_reason, last_login_at, created_at, updated_at) \
         VALUES ($1, 'Dev Operator', 'devoperator', '', $2, '[TBD] Dev Operator', \
         'admin'::user_role, false, '', now(), now(), now()) \
         ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(DEV_LOGIN_USER)
    .bind(DEV_LOGIN_ARMA_ID)
    .execute(&pool)
    .await
    .unwrap_or_else(|e| panic!("prime dev-login row in `{derived_name}`: {e}"));

    pool.close().await;
}
