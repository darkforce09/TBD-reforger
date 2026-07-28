//! Shared support for the `website-api` integration suites (T-334).
//!
//! # This file is NOT a 23rd test binary
//!
//! Cargo builds one test target per **top-level** `tests/*.rs` file. Files under a
//! `tests/<dir>/` subdirectory are not auto-discovered, so this module is compiled *into*
//! whichever suite writes `mod common;` and contributes no target of its own. Measured for
//! this crate with `cargo metadata --no-deps`: **19** `["test"]` targets before this file
//! existed and **19** after. `tests/common/mod.rs` is the conventional home precisely
//! because of that rule; `tests/common.rs` would have become a target.
//!
//! # Why it exists
//!
//! `cargo test -p website-api` builds ~29 test binaries. Until this file, every one of them
//! hand-rolled its own dev-login extractor: 9 copy-pasted
//! `async fn token`/`dev_login`/`admin_token` plus 4 inline copies across 18 files, no `mod`
//! declarations, no `[dev-dependencies]`, no `[[test]]` section in `Cargo.toml`.
//!
//! **Corrected at T-534, measured:** this header used to say those binaries "run
//! concurrently" against one database. Cargo runs test TARGETS one at a time — a run that
//! fails mid-suite stops after the failing binary, and the per-target output blocks never
//! interleave. What *is* concurrent is the tests **inside** one binary. And they no longer
//! share a database at all: [`require_test_database_url`] gives each binary its own.
//!
//! The cost of that duplication is not the duplication. It is that **every copy read the
//! `Location` header through `HeaderMap`'s `Index` impl**, which panics with
//! `no entry found for key "location"` — no status, no body, and no hint at which suite or
//! actor was asking. A dev-login that 404s (route not registered) or 500s (database
//! unreachable, unique-index violation on the shared `users` row) therefore surfaces as a
//! bare key-not-found panic that names neither the cause nor the caller. Under an
//! unattended gate that reads exactly like a code failure, and a fix agent spends its whole
//! retry budget on working code.
//!
//! [`dev_login_token`] replaces that with a sentence.

// Each test binary compiles its own copy of this module and uses a different subset of it,
// so an item unused by *one* suite is not dead code — but rustc cannot know that, and the
// wave gate runs `clippy -p website-api --all-targets -- -D warnings`. Without this, adding
// a helper here for suite A turns suite B red.
#![allow(dead_code)]

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use tower::ServiceExt;
use url::Url;
use uuid::Uuid;
use website_api::state::AppState;

/// Process-local counter for [`unique_arma`]. Starts at 1 so a mint never looks like a bare prefix.
static ARMA_SEQ: AtomicU64 = AtomicU64::new(1);

// ───────────────────────────── T-381 DB target guard ─────────────────────────────

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
/// Measured allow-list (Makefile + `scripts/platform/wave.sh` + operator cold DBs):
/// - `rust_it` — `make test-it` (Makefile DROP/CREATE)
/// - `tbd_gate*` — wave gate (`tbd_gate_w<N>`, `tbd_gate_it`, `tbd_gate_migrate`, …)
/// - `*_cold` — operator `TBD_GATE_DB` cold DBs (`tbd_wave6_cold`, `tbd_t399_cold`, …)
/// - `*_it` / `*_probe` — agent throwaways that already follow the ticket convention
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
             TEST_DATABASE_URL refuses to target database `{name}` (T-381).\n\
             \n  \
             url: {database_url}\n\
             \n  \
             Allowed names: rust_it, tbd_gate*, *_cold, *_it, *_probe.\n  \
             The live dev database `tbd_reforger` is never allowed — pointing the\n  \
             integration suite at it would wipe production-like rows.\n  \
             Fix: `make test-it` (creates rust_it), or export a URL whose path\n  \
             matches the allow-list (wave gate uses tbd_gate_w<N> / *_cold).\n\
             ───────────────────────────────────────────────────────────────────────"
        );
    }
}

// ─────────────────────── T-534 one database per test BINARY ───────────────────────

/// The test binary this copy of `common` was compiled into.
///
/// Cargo compiles one crate per top-level `tests/*.rs`, and `CARGO_CRATE_NAME` is set per
/// compilation unit — so this expands to `admin_field` inside `tests/admin_field.rs`'s
/// binary and to `misc_integration` inside `tests/misc_integration.rs`'s. It is a
/// compile-time `env!`, so a Cargo that stopped setting it is a build error here rather
/// than a silent fallback to one shared name (which is the bug this whole section exists
/// to remove).
const SUITE: &str = env!("CARGO_CRATE_NAME");

/// Resolved once per test binary: the per-binary database URL, or `None` when
/// `TEST_DATABASE_URL` is unset (the suite-skip path).
static PER_BINARY_URL: OnceLock<Option<String>> = OnceLock::new();

/// The shared `dev-login` row's `arma_id`, pinned from `DEV_ARMA_ID` in `src/handlers/dev.rs`.
///
/// Kept honest by [`t534_dev_login_prime_literals_still_match_handler`] — if the handler's
/// literal changes and this one does not, that Class-R goes red instead of the fixture quietly
/// drifting. T-557 moved the stamp out of the INSERT (COALESCE on first create) so the race
/// is gone in the handler; the prime still seeds the same row so ITs match production shape.
const DEV_LOGIN_ARMA_ID: &str = "dev-arma-76561190000000001";

/// Derive this binary's private database name from the operator's base name.
///
/// `("tbd_gate_w60", "admin_field")` → `"tbd_gate_w60_admin_field_it"`.
///
/// The `_it` suffix is not decoration: it is what keeps every generated name inside the
/// T-381 allow-list ([`is_safe_test_database_name`]) no matter what the base was —
/// `rust_it`, `tbd_gate_w60` and `tbd_x_cold` all derive to a `*_it` name. The suffix is
/// re-checked at runtime in [`resolve_and_provision`]; this function is not trusted to have
/// got it right.
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
/// # T-534 — why this no longer returns what the operator exported
///
/// It used to. Every one of the ~25 DB-backed test binaries connected to the single
/// `TEST_DATABASE_URL`, so the suite's verdict depended on what its siblings had left in
/// `users`, `missions`, `user_factions`, … The visible symptom was a wave gate whose result
/// was not reproducible: measured on this tree, 2 of 8 runs — each against its own **fresh
/// cold** database — failed, and earlier runs failed on a *different* test than later ones.
/// A fresh database did not help, because the residue is made **within** one run.
///
/// The cure is one database per binary, created here on first call and named
/// `<base>_<suite>_it` (see [`per_binary_database_name`]). It is dropped and recreated on
/// every run, so a binary's verdict cannot depend on a previous run either, and the T-381
/// allow-list is asserted on the **derived** name as well as the operator's.
///
/// Two things this deliberately does NOT do. It does not serialise anything — tests still
/// run in parallel inside a binary, at full speed. And it does not weaken a single
/// assertion; the suites are unchanged.
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
        "T-534: derived database name `{derived_name}` is not a bare identifier — it is \
         interpolated into DDL below and must never need quoting or escaping"
    );
    let derived_url = with_database_name(&base_url, &derived_name)
        .unwrap_or_else(|| panic!("T-534: cannot rewrite database name in `{base_url}`"));
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
/// `OnceLock` do the once-per-process serialisation with no changes at ~25 call sites.
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
            .expect("T-534: build provisioning runtime")
            .block_on(provision_async(&base_url, &derived_name, &derived_url));
    });
    if let Err(payload) = handle.join() {
        let why = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic payload>");
        panic!("T-534: provisioning tests/{SUITE}.rs's database failed: {why}");
    }
}

async fn provision_async(base_url: &str, derived_name: &str, derived_url: &str) {
    // Maintenance session on the operator's own (allow-listed) database — never `postgres`,
    // so no connection this harness opens is outside the T-381 list. DROP/CREATE DATABASE
    // cannot run against the database being dropped, which is the only reason a second
    // database is involved at all.
    let mut admin = PgConnection::connect(base_url).await.unwrap_or_else(|e| {
        panic!(
            "T-534: connect to `{base_url}` to create tests/{SUITE}.rs's database: {e}\n  \
             The base database must exist — the wave gate's ensure_gate_db creates it, \
             `make test-it` creates rust_it."
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
        .unwrap_or_else(|e| panic!("T-534: DROP DATABASE {derived_name}: {e}"));
    sqlx::raw_sql(AssertSqlSafe(create_sql))
        .execute(&mut admin)
        .await
        .unwrap_or_else(|e| panic!("T-534: CREATE DATABASE {derived_name}: {e}"));
    admin
        .close()
        .await
        .unwrap_or_else(|e| panic!("T-534: close maintenance connection: {e}"));

    let pool = website_api::db::connect(derived_url)
        .await
        .unwrap_or_else(|e| panic!("T-534: connect to `{derived_url}`: {e}"));
    website_api::db::migrate(&pool)
        .await
        .unwrap_or_else(|e| panic!("T-534: migrate `{derived_name}`: {e}"));

    // ── Prime the shared dev-login row. Read this before deleting it. ──
    //
    // Pre-T-557, `dev_login` stamped a FIXED `arma_id` into an INSERT with
    // `ON CONFLICT (discord_id)` only. Concurrent first-time calls raced
    // `idx_users_arma_id` (23505 → 500). T-534 serialised this prime so the IT path
    // never hit an empty `users` table; T-557 fixed the handler (NULL insert +
    // `COALESCE(arma_id, …)` on first create). The prime stays: ITs still want the
    // production row shape (linked arma_id) before any test runs, and it remains
    // defense-in-depth if a future edit reintroduces the race shape (Class-R below).
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
    .unwrap_or_else(|e| panic!("T-534: prime dev-login row in `{derived_name}`: {e}"));

    pool.close().await;
}

/// Class-R (T-534): the derived name is per-binary, stable, and allow-listed.
#[test]
fn t534_per_binary_database_name() {
    // Distinct binaries never derive the same database.
    assert_eq!(
        per_binary_database_name("tbd_gate_w60", "admin_field"),
        "tbd_gate_w60_admin_field_it"
    );
    assert_eq!(
        per_binary_database_name("tbd_gate_w60", "misc_integration"),
        "tbd_gate_w60_misc_integration_it"
    );
    assert_ne!(
        per_binary_database_name("tbd_gate_w60", "admin_field"),
        per_binary_database_name("tbd_gate_w60", "misc_integration")
    );
    // Stable across calls — a name that changed per call would leak a database per run.
    assert_eq!(
        per_binary_database_name("rust_it", "factions"),
        per_binary_database_name("rust_it", "factions")
    );
    // Every base shape in the T-381 allow-list derives to a name the allow-list still takes.
    for base in [
        "rust_it",
        "tbd_gate_it",
        "tbd_gate_w60",
        "tbd_t399_cold",
        "tbd_t350_probe",
    ] {
        let derived = per_binary_database_name(base, "admin_field");
        assert!(
            is_safe_test_database_name(&derived),
            "derived `{derived}` must stay inside the T-381 allow-list"
        );
        assert!(
            derived.len() <= 63,
            "`{derived}` exceeds Postgres' 63 bytes"
        );
    }
    // Over-long input folds into a hash instead of truncating into a shared name.
    let long_a = per_binary_database_name(&"b".repeat(50), &"suite_a".repeat(6));
    let long_b = per_binary_database_name(&"b".repeat(50), &"suite_b".repeat(6));
    assert_eq!(long_a.len(), 63);
    assert_ne!(long_a, long_b, "truncation must not collide two binaries");
    assert!(is_safe_test_database_name(&long_a));
    // Non-identifier characters cannot reach the DDL.
    assert_eq!(
        per_binary_database_name("rust_it", "Odd-Name.42"),
        "rust_it_odd_name_42_it"
    );
    // URL rewrite keeps credentials, host, port and query.
    assert_eq!(
        with_database_name(
            "postgres://tbd:tbd@localhost:5434/tbd_gate_w60?sslmode=disable",
            "tbd_gate_w60_admin_field_it"
        )
        .as_deref(),
        Some("postgres://tbd:tbd@localhost:5434/tbd_gate_w60_admin_field_it?sslmode=disable")
    );
}

/// Class-R (T-534 / T-557): prime literals match handler, and the handler keeps the
/// T-557 race-free contract (fixed `arma_id` via COALESCE first-create, not INSERT stamp).
///
/// The prime seeds the same discord_id / arma_id the handler uses. If either literal
/// drifts, this goes red. T-557 also pins the COALESCE first-create shape and bans the
/// pre-fix INSERT stamp (`'', 'dev-arma-…'`) that raced `idx_users_arma_id`.
#[test]
fn t534_dev_login_prime_literals_still_match_handler() {
    let handler = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers/dev.rs");
    let src = std::fs::read_to_string(&handler)
        .unwrap_or_else(|e| panic!("T-534 Class-R: read {}: {e}", handler.display()));
    for needle in [DEV_LOGIN_USER, DEV_LOGIN_ARMA_ID] {
        assert!(
            src.contains(needle),
            "T-534/T-557: src/handlers/dev.rs no longer contains `{needle}`. The dev-login \
             prime in tests/common/mod.rs seeds that exact row; update both together."
        );
    }
    assert!(
        src.contains("ON CONFLICT (discord_id) DO UPDATE"),
        "T-534/T-557: dev_login's upsert lost ON CONFLICT (discord_id) DO UPDATE — re-derive \
         the race analysis before changing the prime"
    );
    // T-557 contract: arma_id is filled on first create, not stamped into INSERT VALUES.
    assert!(
        src.contains("COALESCE(arma_id"),
        "T-557: expected COALESCE(arma_id, …) first-create path in dev_login — without it, \
         concurrent cold inserts can 23505 on idx_users_arma_id again"
    );
    // Ban the pre-T-557 race shape: quoted fixed arma_id as an INSERT value after avatar ''.
    assert!(
        !src.contains("'', 'dev-arma-76561190000000001'"),
        "T-557: fixed arma_id must not appear as an INSERT VALUES literal after empty \
         avatar_url — that is the concurrent-first-use 23505 shape"
    );
}

/// Class-R: the allow/deny table for [`is_safe_test_database_name`] (T-381).
///
/// Runs inside every binary that `mod common;` — duplicate execution is fine; the
/// pins are pure and cheap.
#[test]
fn t381_test_database_name_guard() {
    // Makefile + wave gate + operator cold.
    assert!(is_safe_test_database_name("rust_it"));
    assert!(is_safe_test_database_name("tbd_gate_it"));
    assert!(is_safe_test_database_name("tbd_gate_w54"));
    assert!(is_safe_test_database_name("tbd_gate_migrate"));
    assert!(is_safe_test_database_name("tbd_wave6_cold"));
    assert!(is_safe_test_database_name("tbd_t399_cold"));
    assert!(is_safe_test_database_name("tbd_t350_probe"));
    assert!(is_safe_test_database_name("tbd_t230_it"));
    // Live / garbage must refuse.
    assert!(!is_safe_test_database_name("tbd_reforger"));
    assert!(!is_safe_test_database_name("postgres"));
    assert!(!is_safe_test_database_name(""));
    assert!(!is_safe_test_database_name("production"));
    // URL parse: path → name.
    assert_eq!(
        database_name_from_url("postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable")
            .as_deref(),
        Some("rust_it")
    );
    assert_eq!(
        database_name_from_url("postgres://tbd:tbd@localhost:5434/tbd_reforger?sslmode=disable")
            .as_deref(),
        Some("tbd_reforger")
    );
}

/// Class-R (T-542): only [`require_test_database_url`] may read `TEST_DATABASE_URL`.
///
/// Scans every top-level `tests/*.rs` binary (not this `common/` module). A raw
/// `env::var("TEST_DATABASE_URL")` outside `common/mod.rs` is a regression — parallel
/// IT against live `tbd_reforger` must panic, not mutate.
#[test]
fn t542_no_raw_test_database_url_reads_outside_common() {
    let needle = concat!("env::var(", "\"TEST_DATABASE_URL\")");
    let tests_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut offenders = Vec::new();
    let entries = std::fs::read_dir(&tests_dir)
        .unwrap_or_else(|e| panic!("T-542 Class-R: read_dir({}): {e}", tests_dir.display()));
    for entry in entries {
        let entry = entry.expect("T-542 Class-R: DirEntry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        // Only top-level binaries — `tests/common/` is not scanned.
        if !path.is_file() {
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("T-542 Class-R: read {}: {e}", path.display()));
        if src.contains(needle) {
            offenders.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(
        offenders.is_empty(),
        "T-542: these IT binaries still contain {needle} — use common::require_test_database_url \
         (only common/mod.rs may hold that literal): {offenders:?}"
    );
}

/// The single identity `GET /auth/dev-login` mints for **every** role
/// (`src/handlers/dev.rs:14`). It is shared by every suite that calls dev-login, and each
/// call rewrites that row's `username`, `discord_handle`, `role` and `last_login_at`
/// (`src/handlers/dev.rs:47-49`) — so a suite must never assume anything about this row
/// beyond what its own most recent dev-login call just wrote.
pub const DEV_LOGIN_USER: &str = "000000000000000001";

/// Mint an access token through `GET /api/v1/auth/dev-login?role={role}`.
///
/// `suite` is the calling file (`"events"`), `role` the tier being requested. Both appear
/// in the failure message, which is the entire point of this helper: on **any** failure it
/// reports the HTTP status, the response body, the URI and who was asking, instead of the
/// `no entry found for key "location"` index panic every hand-rolled copy produced.
///
/// # Correction: `dev_login` has no ban check (T-334, correcting T-365)
///
/// T-365 recorded that dev-login returns no `Location` header because `auth.rs:147` 403s a
/// banned account. **That mechanism is false**, and it has now cost two derivations — do not
/// derive it a third time. Verified against this tree:
///
/// * `src/handlers/dev.rs:25-64` — `dev_login` never reads `is_banned`. Its upsert *writes*
///   `is_banned = false` on insert, and the `ON CONFLICT` branch (`dev.rs:47-49`) does not
///   touch the column at all. It then calls `issue_session` unconditionally.
/// * `src/handlers/auth.rs:35-47` — `issue_session` is `issue_access` + `issue_refresh`.
///   Neither loads the user row, so `is_banned` is never consulted on this path. A banned
///   shared row still gets a **302**.
/// * `src/handlers/auth.rs:144-150` — the ban check lives in `refresh`, i.e.
///   `POST /auth/refresh`, and 403s `"account is banned"` there. That is the line T-365
///   cited; it is simply not on the dev-login path.
///
/// The real mechanism behind a missing `Location` is a **shared-fixture collision**: suites
/// mutate the shared `users` row's role, faction links and `arma_id` out from under each
/// other, and dev-login's response is then not what the next suite expects. Give your suite
/// its own actor ids (see [`seed_user`]) rather than reaching for a ban explanation.
pub async fn dev_login_token(app: &Router, suite: &str, role: &str) -> String {
    let uri = format!("/api/v1/auth/dev-login?role={role}");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&uri)
                .body(Body::empty())
                .expect("build dev-login request"),
        )
        .await
        .expect("dev-login must not fail below HTTP");

    // Read status + Location BEFORE consuming the body: all three go into the message.
    let status = resp.status();
    let location = resp
        .headers()
        .get(header::LOCATION)
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned());
    let body = match to_bytes(resp.into_body(), usize::MAX).await {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => format!("<body unreadable: {e}>"),
    };
    let ctx = DevLoginFailure {
        suite,
        role,
        uri: &uri,
        status,
        body: &body,
        location: location.as_deref(),
    };

    if status != StatusCode::FOUND {
        let msg = ctx.report(
            "expected 302 Found. A non-302 means the dev-login handler did not run: 404 = \
             route not registered (Config::for_tests must set APP_ENV=development), 500 = \
             the handler's users upsert failed (unique index on users.arma_id is the usual \
             cause on a shared integration database).",
        );
        panic!("{msg}");
    }
    let Some(location) = ctx.location else {
        let msg = ctx.report(
            "302 with no Location header. The redirect was built by something other than \
             session_redirect (src/handlers/auth.rs:96-103).",
        );
        panic!("{msg}");
    };
    let Some((_, fragment)) = location.split_once('#') else {
        let msg = ctx.report(
            "Location carries no `#` fragment. auth_callback_url puts the tokens in the \
             fragment (src/handlers/auth.rs:82-94); a fragment-less Location is an error \
             redirect, and its `error=` query names the reason.",
        );
        panic!("{msg}");
    };
    let Some(token) = fragment
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
    else {
        let msg = ctx.report("the Location fragment carries no `access_token=` pair.");
        panic!("{msg}");
    };
    token.to_string()
}

/// Everything known about a failed dev-login, so the panic can name it.
struct DevLoginFailure<'a> {
    suite: &'a str,
    role: &'a str,
    uri: &'a str,
    status: StatusCode,
    body: &'a str,
    location: Option<&'a str>,
}

impl DevLoginFailure<'_> {
    fn report(&self, why: &str) -> String {
        let Self {
            suite,
            role,
            uri,
            status,
            body,
            location,
        } = *self;
        let body = if body.is_empty() {
            "<empty>".to_string()
        } else if body.len() > 2000 {
            format!("{}… ({} bytes total)", &body[..2000], body.len())
        } else {
            body.to_string()
        };
        let location = location.unwrap_or("<absent>");
        format!(
            "\n\
             ───────────────────────────────────────────────────────────────────────\n\
             dev-login did not mint a session.\n\
             \n  \
             suite:    tests/{suite}.rs\n  \
             actor:    dev-login shared user {DEV_LOGIN_USER}, role={role}\n  \
             request:  GET {uri}\n  \
             status:   {status}\n  \
             location: {location}\n  \
             body:     {body}\n\
             \n  \
             {why}\n\
             ───────────────────────────────────────────────────────────────────────"
        )
    }
}

/// Mint an `arma_id` that cannot collide with a parallel `seed_user` in this process.
///
/// `idx_users_arma_id` is a global non-partial unique index. A fixed string like
/// `events-arma-{discord_id}` is a single slot: two concurrent seeds (same or different
/// discord rows racing through release/upsert) can still trip it. Prefer this over sleep.
///
/// Format: `{prefix}-{seq}-{uuid}` — seq is monotonic in-process; uuid covers cross-binary
/// overlap on a shared gate DB (each test binary gets its own `ARMA_SEQ`).
pub fn unique_arma(prefix: &str) -> String {
    let n = ARMA_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{n}-{}", Uuid::new_v4())
}

/// Seed one user row this suite owns outright.
///
/// `arma_id` is a required argument and must be **unique across the whole database**:
/// `idx_users_arma_id` (`migrations/0001_initial_schema.sql:887`) is a plain
/// `CREATE UNIQUE INDEX`, not a partial one, so `''` is a *value* and only one row in the
/// entire integration database may hold it. Passing `''` here is the collision waiting to
/// happen — two suites doing it for different `discord_id`s is a unique violation on the
/// second, and `ON CONFLICT (discord_id)` does not catch it because the conflict is on the
/// other index. NULL would coexist, but a distinct string is cheaper to trace in a failure.
///
/// # T-479 — idempotent on `arma_id`
///
/// `ON CONFLICT (discord_id)` only absorbs a discord_id clash. If **another** row already
/// holds `arma_id`, the `DO UPDATE SET arma_id = EXCLUDED.arma_id` branch raises
/// `unique_violation` on `idx_users_arma_id` (observed: `event_orbat_registration_and_race`
/// / `events-arma-000000000000334002` under parallel IT). Before the upsert we release any
/// foreign holder of this `arma_id` inside the same transaction, so a leftover or racing
/// placeholder cannot poison the seed. Suites that share fixed placeholders across parallel
/// tests should still serialise (T-516 Mutex) **or** mint via [`unique_arma`].
///
/// `ON CONFLICT DO UPDATE`, not `DO NOTHING`: a suite that owns its ids wants the fixture it
/// asked for on every run, not whatever the previous run happened to leave behind.
pub async fn seed_user(pool: &PgPool, discord_id: &str, username: &str, arma_id: &str, role: &str) {
    let mut tx = pool.begin().await.unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): begin: {e}")
    });

    // Free the arma slot from any *other* discord_id so the upsert cannot unique-violate.
    sqlx::query(
        "UPDATE users SET arma_id = NULL, updated_at = now() \
         WHERE arma_id = $1 AND discord_id IS DISTINCT FROM $2",
    )
    .bind(arma_id)
    .bind(discord_id)
    .execute(&mut *tx)
    .await
    .unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}): release foreign holder: {e}")
    });

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
         arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, $2, $2, '', $3, '', $4::user_role, false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET \
          username = EXCLUDED.username, arma_id = EXCLUDED.arma_id, role = EXCLUDED.role, \
          is_banned = false, updated_at = now()",
    )
    .bind(discord_id)
    .bind(username)
    .bind(arma_id)
    .bind(role)
    .execute(&mut *tx)
    .await
    .unwrap_or_else(|e| panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): {e}"));

    tx.commit().await.unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): commit: {e}")
    });
}

/// Mint an access token **without** rewriting the shared `dev-login` row.
///
/// Prefer this whenever the suite needs a specific `discord_id` (private actor) or must not
/// leave `users.role` on `DEV_LOGIN_USER` as `enlisted` for a sibling binary that reads the
/// DB (`misc_integration.rs` asserts `role == admin` via `GET /me`). JWT role gates
/// (`MissionMakerUser`, `AdminUser`, …) read the claim, not the row — so this loses no
/// coverage versus `dev_login_token` for authz paths.
///
/// `suite` appears in the panic so a mint failure names the caller the same way
/// [`dev_login_token`] does.
pub fn access_token(
    state: &AppState,
    suite: &str,
    discord_id: &str,
    role: &str,
    arma_linked: bool,
) -> String {
    state
        .jwt
        .issue_access(discord_id, role, arma_linked)
        .unwrap_or_else(|e| {
            panic!(
                "tests/{suite}.rs: issue_access(discord_id={discord_id}, role={role}, \
                 arma_linked={arma_linked}): {e}"
            )
        })
        .0
}
