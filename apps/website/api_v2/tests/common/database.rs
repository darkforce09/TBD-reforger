//! Per-binary integration database: target guard, name derivation, and provisioning.
//!
//! Every DB-backed suite calls [`require_test_database_url`] instead of reading
//! `TEST_DATABASE_URL` itself. That single entry point is what makes two guarantees hold for
//! all of them at once: the URL can only ever point at an allow-listed throwaway database,
//! and each test binary gets a private one so a suite's verdict cannot depend on rows a
//! sibling binary left behind.

use std::sync::OnceLock;

use sqlx::{AssertSqlSafe, Connection, PgConnection};
use url::Url;

use super::http::{DEV_LOGIN_ARMA_ID, DEV_LOGIN_USER};

// ───────────────────────────── target guard ─────────────────────────────

/// Extract the PostgreSQL database name from a connection URL's path.
///
/// `postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable` → `Some("rust_it")`.
/// Empty path, unparseable URL, or a multi-segment path → `None`.
pub fn database_name_from_url(database_url: &str) -> Option<String> {
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
pub fn is_safe_test_database_name(name: &str) -> bool {
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
/// migrate / any DELETE). Unset URL is the caller's skip path — this only runs when
/// a URL is present.
pub fn assert_test_database_url(database_url: &str) {
    let name = database_name_from_url(database_url).unwrap_or_else(|| {
        panic!(
            "\n\
             ───────────────────────────────────────────────────────────────────────\n\
             TEST_DATABASE_URL is set but its database name could not be parsed.\n\
             \n  \
             url: {database_url}\n\
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
             url: {database_url}\n\
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
/// Cargo compiles one crate per top-level `tests/*.rs`, and `CARGO_CRATE_NAME` is set per
/// compilation unit — so this expands to `servers_crud` inside `tests/servers_crud.rs`'s
/// binary and to `dev_login_runtime_identity` inside `tests/dev_login_runtime_identity.rs`'s. It is a
/// compile-time `env!`, so a Cargo that stopped setting it is a build error here rather
/// than a silent fallback to one shared name — which is the defect this whole section
/// exists to prevent.
const SUITE: &str = env!("CARGO_CRATE_NAME");

/// Resolved once per test binary: the per-binary database URL, or `None` when
/// `TEST_DATABASE_URL` is unset (the suite-skip path).
static PER_BINARY_URL: OnceLock<Option<String>> = OnceLock::new();

/// Derive this binary's private database name from the operator's base name.
///
/// `("rust_it", "servers_crud")` → `"rust_it_servers_crud_it"`.
///
/// The `_it` suffix is not decoration: it is what keeps every generated name inside the
/// allow-list ([`is_safe_test_database_name`]) no matter what the base was — `rust_it`,
/// `tbd_gate_migrate` and `tbd_x_cold` all derive to a `*_it` name. The suffix is re-checked at
/// runtime in [`resolve_and_provision`]; this function is not trusted to have got it right.
///
/// Postgres truncates identifiers at 63 bytes, and a truncated name is a name two binaries
/// can share — which is exactly the defect. Over-long names therefore fold their tail into a
/// hash rather than losing it.
pub fn per_binary_database_name(base: &str, suite: &str) -> String {
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
pub fn with_database_name(url: &str, database: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    parsed.set_path(database);
    Some(parsed.into())
}

/// Read `TEST_DATABASE_URL` and hand back **this binary's own** database URL.
///
/// `None` when unset (suite skip); panics when the operator's URL — or the name derived
/// from it — is not an allow-listed test database.
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
/// Two things this deliberately does NOT do. It does not serialise anything — tests still
/// run in parallel inside a binary, at full speed. And it does not weaken a single
/// assertion; the suites are unchanged.
///
/// # Known limit: concurrent `cargo test` processes still race
///
/// Two concurrent `cargo test` invocations against the **same** operator base race on
/// `<base>_<suite>_it`: both binaries run [`provision`]'s `DROP DATABASE … WITH (FORCE)` /
/// `CREATE DATABASE` for the same derived name. `cargo xtask db test-it` runs the binaries one
/// after another, so that path is covered; cross-process overlap outside it would need
/// PID-suffixed names or a cross-process provision lock.
pub fn require_test_database_url() -> Option<String> {
    PER_BINARY_URL.get_or_init(resolve_and_provision).clone()
}

/// One-shot: derive the per-binary name, guard it, create the database, migrate it, and
/// prime the shared `dev-login` row. Runs at most once per test binary.
fn resolve_and_provision() -> Option<String> {
    let base_url = std::env::var("TEST_DATABASE_URL").ok()?;
    // The operator's own name is checked first and unchanged — a URL pointing at the live
    // database must still panic here, before anything below can create or drop anything.
    assert_test_database_url(&base_url);
    let base_name =
        database_name_from_url(&base_url).expect("assert_test_database_url accepted the URL");

    let derived_name = per_binary_database_name(&base_name, SUITE);
    assert!(
        derived_name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "derived database name `{derived_name}` is not a bare identifier — it is \
         interpolated into DDL below and must never need quoting or escaping"
    );
    let derived_url = with_database_name(&base_url, &derived_name)
        .unwrap_or_else(|| panic!("cannot rewrite database name in `{base_url}`"));
    // The guard applies to what we actually connect to, not only to what was exported.
    // If a future base name derives to something the allow-list refuses, that is a hard
    // stop here rather than a database created outside the list.
    assert_test_database_url(&derived_url);

    provision(&base_url, &derived_name, &derived_url);
    Some(derived_url)
}

/// Run [`provision_async`] on its own thread + runtime.
///
/// `require_test_database_url` is called from inside `#[tokio::test]` bodies, so it cannot
/// `block_on` here — a nested `block_on` panics. A dedicated thread owning its own
/// current-thread runtime keeps the caller's signature synchronous, which is what lets
/// `OnceLock` do the once-per-process serialisation with no changes at the call sites.
fn provision(base_url: &str, derived_name: &str, derived_url: &str) {
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
        panic!("provisioning tests/{SUITE}.rs's database failed: {why}");
    }
}

async fn provision_async(base_url: &str, derived_name: &str, derived_url: &str) {
    // Maintenance session on the operator's own (allow-listed) database — never `postgres`,
    // so no connection this harness opens is outside the allow-list. DROP/CREATE DATABASE
    // cannot run against the database being dropped, which is the only reason a second
    // database is involved at all.
    let mut admin = PgConnection::connect(base_url).await.unwrap_or_else(|e| {
        panic!(
            "connect to `{base_url}` to create tests/{SUITE}.rs's database: {e}\n  \
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

    let pool = website_api::core::database::connect(derived_url)
        .await
        .unwrap_or_else(|e| panic!("connect to `{derived_url}`: {e}"));
    website_api::core::database::migrate(&pool)
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

/// Assert that only [`require_test_database_url`] reads `TEST_DATABASE_URL`.
///
/// Scans every top-level `tests/*.rs` binary (not this `common/` module) **and** every
/// `src/**/*.rs` file. Scanning `src/` is not optional: an in-crate `#[tokio::test]` can read
/// the operator base raw and stay invisible to a tests-only scan. A raw
/// `env::var("TEST_DATABASE_URL")` outside this module is a regression — parallel integration
/// runs against live `tbd_reforger` must panic, not mutate.
pub fn assert_no_raw_test_database_url_reads_outside_common() {
    let needle = concat!("env::var(", "\"TEST_DATABASE_URL\")");
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();

    // Top-level integration binaries — `tests/common/` is not scanned.
    let tests_dir = manifest.join("tests");
    let entries = std::fs::read_dir(&tests_dir)
        .unwrap_or_else(|e| panic!("read_dir({}): {e}", tests_dir.display()));
    for entry in entries {
        let entry = entry.expect("DirEntry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") || !path.is_file() {
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if src.contains(needle) {
            offenders.push(format!(
                "tests/{}",
                path.file_name().unwrap().to_string_lossy()
            ));
        }
    }

    // Walk `src/` so lib-target DB tests cannot hide from this pin.
    let src_dir = manifest.join("src");
    fn walk_rs(
        dir: &std::path::Path,
        needle: &str,
        offenders: &mut Vec<String>,
        root: &std::path::Path,
    ) {
        let entries =
            std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir({}): {e}", dir.display()));
        for entry in entries {
            let entry = entry.expect("DirEntry");
            let path = entry.path();
            if path.is_dir() {
                walk_rs(&path, needle, offenders, root);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            if src.contains(needle) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                offenders.push(rel);
            }
        }
    }
    walk_rs(&src_dir, needle, &mut offenders, manifest);

    assert!(
        offenders.is_empty(),
        "these paths still contain {needle} — use common::require_test_database_url \
         (only tests/common/database.rs may hold that literal): {offenders:?}"
    );
}
