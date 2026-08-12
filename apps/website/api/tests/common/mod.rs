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
/// - `rust_it` — `cargo xtask db test-it` (Makefile DROP/CREATE)
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
             Fix: `cargo xtask db test-it` (creates rust_it), or export a URL whose path\n  \
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
///
/// # T-558 finding 4 — concurrent `cargo test` processes still race
///
/// Two concurrent `cargo test` invocations against the **same** operator base still race on
/// `<base>_<suite>_it`: both binaries run [`provision`]'s `DROP DATABASE … WITH (FORCE)` /
/// `CREATE DATABASE` for the same derived name. That is no worse than the pre-T-534 race on
/// `<base>` itself. The wave gate's flock already serialises gate runs, so the gate path is
/// covered. Fixing cross-process overlap (PID-suffixed names, or a cross-process provision
/// lock) is outside this slice's `owns` — recorded here so it is not rediscovered as new.
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
             `cargo xtask db test-it` creates rust_it."
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

/// Index just past the Rust literal that opens at `bytes[i]`, or `None` when none does.
///
/// Handles `"…"`, `r"…"` / `r#"…"#` (any hash count), the `b` / `c` prefixes, and char
/// literals `'x'` / `'\n'`. A `'` that introduces a **lifetime** (`&'static str`) opens
/// nothing — that is the T-569 finding, and this module's own copy of the comment
/// stripper still had the bug it names: on `src/handlers/dev.rs` the `'` of
/// `-> &'static str` opened a char span that ran to the `'` of `'Dev Operator'` sixty
/// lines later, so every `//` comment in between survived "comment-stripped" source and
/// brace depth over that span was fiction. [`rust_fn_body`] now needs that depth to be
/// real (T-571), so the lexing is shared rather than approximated twice.
fn rust_literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    let n = bytes.len();
    if i >= n {
        return None;
    }
    // Char literal vs lifetime: `'a` / `'static` are lifetimes; `'x'` and `'\n'` are not.
    if bytes[i] == b'\'' {
        let escaped = bytes.get(i + 1) == Some(&b'\\');
        let single = bytes.get(i + 2) == Some(&b'\'');
        if !escaped && !single {
            return None; // lifetime (or a lone quote) — consumes nothing
        }
        let mut j = i + 1;
        while j < n {
            if bytes[j] == b'\\' && j + 1 < n {
                j += 2;
                continue;
            }
            if bytes[j] == b'\'' {
                return Some(j + 1);
            }
            j += 1;
        }
        return Some(n);
    }
    // Not mid-identifier (`foo_r"…"` is not a raw string at the `r`).
    if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        return None;
    }
    let mut p = i;
    if matches!(bytes.get(p), Some(b'b' | b'c')) && matches!(bytes.get(p + 1), Some(b'r' | b'"')) {
        p += 1;
    }
    let mut hashes = 0usize;
    let raw = bytes.get(p) == Some(&b'r');
    if raw {
        p += 1;
        while bytes.get(p) == Some(&b'#') {
            hashes += 1;
            p += 1;
        }
    }
    if bytes.get(p) != Some(&b'"') {
        return None;
    }
    let mut j = p + 1;
    if raw {
        while j < n {
            if bytes[j] == b'"' && (1..=hashes).all(|h| bytes.get(j + h) == Some(&b'#')) {
                return Some(j + 1 + hashes);
            }
            j += 1;
        }
        return Some(n);
    }
    while j < n {
        if bytes[j] == b'\\' && j + 1 < n {
            j += 2;
            continue;
        }
        if bytes[j] == b'"' {
            return Some(j + 1);
        }
        j += 1;
    }
    Some(n)
}

/// Strip `//` line and `/* */` block comments outside string/char literals.
///
/// T-560: Class-R that greps raw source for `COALESCE(arma_id` is hollow — a comment
/// alone keeps the pin green while the live UPDATE is gone. Always pin against this.
///
/// T-562 / T-565 / T-568: comment strip alone is still hollow — string / `format!`
/// decoys, SELECT + SQL-comment payloads, dead helpers retaining the COALESCE UPDATE,
/// and needles parked inside SQL string literals keep a naive `contains` green. Use
/// [`sqlx_queries_have_arma_coalesce_update`] on payloads from the live first-create
/// fn ([`rust_fn_body`] of `dev_login`) after SQL-comment + string-literal strip.
///
/// T-571: literal spans come from [`rust_literal_end`] now — raw strings are copied
/// whole (a `//` inside `r#"…"#` is text, not a comment) and a lifetime no longer opens
/// a sixty-line char span across the file.
fn strip_rust_comments_outside_literals(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(end) = rust_literal_end(bytes, i) {
            for &b in &bytes[i..end] {
                out.push(b as char);
            }
            i = end;
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Byte length of the PostgreSQL dollar-quote delimiter opening at `bytes[i]`
/// (`$$` → 2, `$decoy$` → 7), or `None` when one does not open there.
///
/// **T-571.** `$$…$$` / `$tag$…$tag$` is a string literal in PostgreSQL exactly as `'…'`
/// is, and [`blank_sql_string_literal_contents`] knew only about `'` and `"` — so
/// `sqlx::query("SELECT $d$UPDATE users SET arma_id = COALESCE(arma_id, $2)$d$")` reached
/// the needle match with its payload intact and kept the pin green over a SELECT.
///
/// The tag rule is PostgreSQL's own: empty, or `[A-Za-z_][A-Za-z0-9_]*`. That is what
/// keeps the crate's real `$1` / `$2` bind placeholders from being read as delimiters —
/// a digit cannot start a tag, so `$2), updated_at` is not an opener and the live UPDATE
/// is unaffected.
fn pg_dollar_delim_len(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'$') {
        return None;
    }
    if bytes.get(i + 1) == Some(&b'$') {
        return Some(2);
    }
    let mut j = i + 1;
    if !bytes
        .get(j)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
    {
        return None;
    }
    while bytes
        .get(j)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    {
        j += 1;
    }
    if bytes.get(j) == Some(&b'$') {
        Some(j + 1 - i)
    } else {
        None
    }
}

/// End index (exclusive) of the dollar-quoted literal opening at `i`, or `None` when the
/// delimiter is never repeated — an unclosed `$tag$` is not a literal, so it is left alone
/// rather than swallowing the rest of the payload.
fn pg_dollar_literal_end(bytes: &[u8], i: usize, delim_len: usize) -> Option<usize> {
    let delim = &bytes[i..i + delim_len];
    let mut j = i + delim_len;
    while j + delim_len <= bytes.len() {
        if &bytes[j..j + delim_len] == delim {
            return Some(j + delim_len);
        }
        j += 1;
    }
    None
}

/// Strip SQL `--` line and `/* */` block comments outside string literals.
///
/// T-565: a `sqlx::query("SELECT 1 -- SET arma_id = COALESCE…")` decoy kept the T-562
/// pin green after the real UPDATE was deleted. Comment-strip the SQL payload before
/// matching the first-create UPDATE shape.
///
/// T-571: dollar-quoted literals are copied whole, so a `--` *inside* one cannot eat its
/// closing delimiter and leave the literal unrecognised by the blanker downstream.
fn strip_sql_comments_outside_literals(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    let mut in_quote: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if in_quote.is_none()
            && let Some(dl) = pg_dollar_delim_len(bytes, i)
            && let Some(end) = pg_dollar_literal_end(bytes, i, dl)
        {
            for &b in &bytes[i..end] {
                out.push(b as char);
            }
            i = end;
            continue;
        }
        if let Some(q) = in_quote {
            out.push(c as char);
            if c == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if c == q {
                in_quote = None;
            }
            i += 1;
            continue;
        }
        if c == b'\'' || c == b'"' {
            in_quote = Some(c);
            out.push(c as char);
            i += 1;
            continue;
        }
        if c == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Blank interiors of SQL `'…'` / `"…"` / `$tag$…$tag$` literals (keep the delimiters).
///
/// T-568 W66 DIRTY MAJOR: after T-565's comment strip, a SELECT whose WHERE clause
/// embeds `'UPDATE users SET arma_id = COALESCE…'` still matched the UPDATE needle —
/// string-literal contents survive comment strip. Blank them before the shape check.
///
/// T-571 W67 DIRTY MAJOR: the same decoy in PostgreSQL's *other* string-literal syntax
/// walked straight through, because this function knew only the two quote characters.
/// `$decoy$UPDATE users SET arma_id = COALESCE(arma_id, $2)$decoy$` inside a SELECT kept
/// the pin green with no live UPDATE anywhere. Dollar quoting is handled here now (see
/// [`pg_dollar_delim_len`] for why `$1` / `$2` binds are not delimiters).
fn blank_sql_string_literal_contents(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(dl) = pg_dollar_delim_len(bytes, i)
            && let Some(end) = pg_dollar_literal_end(bytes, i, dl)
        {
            for &b in &bytes[i..i + dl] {
                out.push(b as char);
            }
            for _ in i + dl..end - dl {
                out.push(' ');
            }
            for &b in &bytes[end - dl..end] {
                out.push(b as char);
            }
            i = end;
            continue;
        }
        if c == b'\'' || c == b'"' {
            let q = c;
            out.push(c as char);
            i += 1;
            while i < bytes.len() {
                let d = bytes[i];
                // SQL standard: doubled quote escapes inside the same delimiter.
                if d == q && i + 1 < bytes.len() && bytes[i + 1] == q {
                    i += 2;
                    continue;
                }
                if d == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if d == q {
                    out.push(d as char);
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Collapse whitespace so `UPDATE  users\\nSET…` matches the product UPDATE shape.
fn flatten_sql_ws(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Brace-balanced body of the **file-scope** item `fn {name}` in comment-stripped source
/// (opening `{` … matching `}`).
///
/// T-568: the COALESCE pin must bind to the live first-create path (`dev_login`), not a
/// dead sibling helper that retains the UPDATE while the executed path uses `$2`.
///
/// # T-571 — first textual match was not the item
///
/// W67 walked around T-568 with a *nested* definition:
///
/// ```text
/// mod decoy { async fn dev_login() { sqlx::query("UPDATE users SET arma_id = COALESCE…"); } }
/// pub async fn dev_login(…) { …live path uses `SET arma_id = $2`… }
/// ```
///
/// `str::find` returns the decoy, so the pin read the decoy's body, found the needle, and
/// reported success over source it never examined — with the live UPDATE gone. The fix is
/// not a ban on the word `mod`: the pin *meant* the file-scope item all along, so this
/// counts braces (skipping literals via [`rust_literal_end`]) and considers only markers at
/// **depth 0**. Anything nested inside a `mod`, `impl`, `fn` or block is not the item the
/// crate calls, by construction rather than by enumeration.
///
/// Requiring exactly one such definition costs nothing — two file-scope `fn dev_login`s do
/// not compile — and turns "which one did it read?" into a named failure.
fn rust_fn_body<'a>(code: &'a str, name: &str) -> &'a str {
    let marker = format!("fn {name}");
    let m = marker.as_bytes();
    let bytes = code.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    let mut starts: Vec<usize> = Vec::new();
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {
                if depth == 0
                    && bytes[i..].starts_with(m)
                    && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'))
                    && bytes
                        .get(i + m.len())
                        .is_none_or(|c| !(c.is_ascii_alphanumeric() || *c == b'_'))
                {
                    starts.push(i);
                }
            }
        }
        i += 1;
    }
    let start = match starts.as_slice() {
        [only] => *only,
        [] => panic!(
            "T-568/T-571 Class-R: no FILE-SCOPE `{marker}` in comment-stripped source. A \
             definition nested in a `mod`/`impl`/block is not the item the crate calls — the \
             pin binds to the top-level one on purpose."
        ),
        many => panic!(
            "T-571 Class-R: {} file-scope `{marker}` definitions (offsets {many:?}) — a Rust \
             file cannot have two, so this source is not what the crate compiles.",
            many.len()
        ),
    };
    let after = &code[start..];
    let open = after
        .find('{')
        .unwrap_or_else(|| panic!("T-568 Class-R: `{marker}` has no opening brace"));
    let bytes = after.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        if bytes[i] == b'{' {
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return &after[open..=i];
            }
        }
        i += 1;
    }
    panic!("T-568 Class-R: `{marker}` body not closed");
}

/// True when a direct `sqlx::query("…")` payload is the first-create arma UPDATE
/// (`UPDATE users SET arma_id = COALESCE(arma_id…`), not a SELECT/comment/string decoy.
///
/// T-565: any-payload `contains("SET arma_id = COALESCE(arma_id")` was hollow — a SELECT
/// with the needle in a SQL `--` comment stayed green. Require the UPDATE statement shape
/// after SQL-comment strip.
///
/// T-568: also blank SQL string-literal interiors — a SELECT with the needle inside
/// `'UPDATE users SET arma_id = COALESCE…'` survived comment strip alone.
fn sqlx_payload_is_arma_coalesce_update(payload: &str) -> bool {
    let comments_gone = strip_sql_comments_outside_literals(payload);
    let literals_blank = blank_sql_string_literal_contents(&comments_gone);
    let flat = flatten_sql_ws(&literals_blank);
    flat.contains("UPDATE users SET arma_id = COALESCE(arma_id")
}

fn sqlx_queries_have_arma_coalesce_update(payloads: &[String]) -> bool {
    payloads
        .iter()
        .any(|p| sqlx_payload_is_arma_coalesce_update(p))
}

/// Interiors of `"…"` / concatenated `"…" "…"` literals that are *direct* args to
/// `sqlx::query(`. Skips `format!("…")`, `let _decoy = "…"`, and `&format!(…)` wrappers.
///
/// T-562: the live COALESCE path is SQL text inside `sqlx::query("…")`. A decoy string
/// or `format!` with the same needle must not satisfy the Class-R pin.
fn sqlx_query_string_payloads(code: &str) -> Vec<String> {
    let bytes = code.as_bytes();
    let key = b"sqlx::query";
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + key.len() <= bytes.len() {
        if &bytes[i..i + key.len()] != key {
            i += 1;
            continue;
        }
        let before_ok = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        let mut j = i + key.len();
        // Reject `sqlx::query_as` / `sqlx::query_scalar` — only bare `sqlx::query(`.
        if j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
            i = j;
            continue;
        }
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if !before_ok || j >= bytes.len() || bytes[j] != b'(' {
            i += key.len();
            continue;
        }
        j += 1; // past '('
        let mut payload = String::new();
        loop {
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= bytes.len() || bytes[j] != b'"' {
                break;
            }
            j += 1; // opening "
            while j < bytes.len() {
                let c = bytes[j];
                if c == b'\\' && j + 1 < bytes.len() {
                    let n = bytes[j + 1];
                    // Rust string line-continuation: `\` + newline (optional CR).
                    if n == b'\n' {
                        j += 2;
                        continue;
                    }
                    if n == b'\r' && j + 2 < bytes.len() && bytes[j + 2] == b'\n' {
                        j += 3;
                        continue;
                    }
                    payload.push(n as char);
                    j += 2;
                    continue;
                }
                if c == b'"' {
                    j += 1;
                    break;
                }
                payload.push(c as char);
                j += 1;
            }
        }
        if !payload.is_empty() {
            out.push(payload);
        }
        i = j.max(i + key.len());
    }
    out
}

#[test]
fn t562_sqlx_query_payloads_ignore_string_and_format_decoys() {
    let live = r#"
        sqlx::query(
            "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
             WHERE discord_id = $1",
        )
        .bind(DEV_USER_ID)
        .bind(DEV_ARMA_ID);
    "#;
    let payloads = sqlx_query_string_payloads(live);
    assert!(
        sqlx_queries_have_arma_coalesce_update(&payloads),
        "live sqlx::query UPDATE must surface the COALESCE needle; got {payloads:?}"
    );

    let decoy = r#"
        let _decoy = "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() WHERE discord_id = $1";
        let _fmt = format!("SET arma_id = COALESCE(arma_id, {})", "x");
        sqlx::query("INSERT INTO users (arma_id) VALUES (NULL)");
    "#;
    let hollow = sqlx_query_string_payloads(decoy);
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&hollow),
        "T-562: string/format! decoys must not count as live COALESCE; got {hollow:?}"
    );

    // format!-wrapped query arg is not a direct string literal to sqlx::query(
    let wrapped = r#"sqlx::query(&format!("SET arma_id = COALESCE(arma_id, {})", "x"));"#;
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(wrapped)),
        "T-562: format!-wrapped sqlx::query must not satisfy the pin"
    );
}

#[test]
fn t565_sqlx_query_coalesce_requires_update_not_select_comment() {
    // W65 DIRTY MAJOR: T-562 grepped any sqlx::query payload for the needle — a SELECT
    // with the needle in a SQL `--` comment stayed green after the real UPDATE was deleted.
    let select_comment = r#"
        sqlx::query(
            "SELECT 1 -- SET arma_id = COALESCE(arma_id, $2) decoy",
        )
        .execute(&state.pool);
    "#;
    let hollow = sqlx_query_string_payloads(select_comment);
    assert!(
        hollow
            .iter()
            .any(|p| p.contains("SET arma_id = COALESCE(arma_id")),
        "fixture sanity: SELECT-comment decoy still embeds the raw needle in the payload"
    );
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&hollow),
        "T-565: SELECT + SQL-comment decoy must not count as first-create UPDATE; got {hollow:?}"
    );

    let block_comment =
        r#"sqlx::query("SELECT 1 /* UPDATE users SET arma_id = COALESCE(arma_id, $2) */");"#;
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(block_comment)),
        "T-565: SELECT + /* */ UPDATE decoy must not satisfy the pin"
    );

    let live = r#"
        sqlx::query(
            "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
             WHERE discord_id = $1",
        );
    "#;
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(live)),
        "T-565: real first-create UPDATE must still satisfy the pin"
    );
}

#[test]
fn t568_coalesce_pin_rejects_dead_helper_and_sql_string_literal() {
    // W66 DIRTY MAJOR after T-565 — attack 1: COALESCE parked in an unused sibling fn
    // while the live first-create path uses `SET arma_id = $2`. Whole-file payload scan
    // stayed GREEN; pin must bind to `dev_login` only.
    let dead_helper = r#"
        fn unused_arma_coalesce_helper() {
            let _ = sqlx::query(
                "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
                 WHERE discord_id = $1",
            );
        }
        pub async fn dev_login() {
            sqlx::query(
                "UPDATE users SET arma_id = $2, updated_at = now() WHERE discord_id = $1",
            );
        }
    "#;
    let code = strip_rust_comments_outside_literals(dead_helper);
    // Sanity: whole-file scan still sees the dead helper (the pre-T-568 hollow shape).
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(&code)),
        "fixture sanity: dead helper still embeds a COALESCE UPDATE payload at file scope"
    );
    let login_payloads = sqlx_query_string_payloads(rust_fn_body(&code, "dev_login"));
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&login_payloads),
        "T-568: dead-helper COALESCE + live `SET arma_id = $2` must go RED when pinned to \
         dev_login; got {login_payloads:?}"
    );

    // Attack 2: needle inside a SQL string literal survives comment strip.
    let string_lit = r#"
        pub async fn dev_login() {
            sqlx::query(
                "SELECT 1 WHERE 'UPDATE users SET arma_id = COALESCE(arma_id, $2)' = 'x'",
            );
        }
    "#;
    let lit_code = strip_rust_comments_outside_literals(string_lit);
    let lit_payloads = sqlx_query_string_payloads(rust_fn_body(&lit_code, "dev_login"));
    assert!(
        lit_payloads
            .iter()
            .any(|p| p.contains("UPDATE users SET arma_id = COALESCE(arma_id")),
        "fixture sanity: SQL string-literal decoy still embeds the raw needle in the payload"
    );
    // Pre-blank hollow: comment-strip alone would still match (W66 measured).
    assert!(
        lit_payloads.iter().any(|p| {
            let flat = flatten_sql_ws(&strip_sql_comments_outside_literals(p));
            flat.contains("UPDATE users SET arma_id = COALESCE(arma_id")
        }),
        "fixture sanity: comment-strip alone still sees the string-literal needle (W66 hollow)"
    );
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&lit_payloads),
        "T-568: SELECT with COALESCE needle inside a SQL string literal must go RED; \
         got {lit_payloads:?}"
    );

    // T-565 SELECT -- comment decoy must remain RED under the hardened pin.
    let select_comment = r#"
        pub async fn dev_login() {
            sqlx::query(
                "SELECT 1 -- SET arma_id = COALESCE(arma_id, $2) decoy",
            );
        }
    "#;
    let sc = strip_rust_comments_outside_literals(select_comment);
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(rust_fn_body(
            &sc,
            "dev_login"
        ))),
        "T-568: T-565 SELECT -- comment decoy must stay RED"
    );

    // Live first-create path inside `dev_login` must stay GREEN.
    let live = r#"
        pub async fn dev_login() {
            sqlx::query(
                "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
                 WHERE discord_id = $1",
            );
        }
    "#;
    let live_code = strip_rust_comments_outside_literals(live);
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(rust_fn_body(
            &live_code,
            "dev_login"
        ))),
        "T-568: live COALESCE UPDATE inside dev_login must still satisfy the pin"
    );
}

/// T-571: the two W67 attacks the ticket names, and the two invented while fixing them.
///
/// Attacks 1 and 2 are **lexical** defects and are fixed here, because a lexer that mistakes
/// a nested item for a file-scope one, or misses one of PostgreSQL's two string-literal
/// syntaxes, is simply wrong about its input. Attacks 4 and 5 are **reachability** defects
/// and are *not* fixed here — see [`t534_dev_login_prime_literals_still_match_handler`] for
/// the shapes this pin still admits and for the runtime test that is the actual authority.
#[test]
fn t571_pin_binds_file_scope_dev_login_and_blanks_pg_dollar_quotes() {
    // ── T-571 attack 1 (named): a nested `fn dev_login` decoy wins `str::find` ────────
    let nested = r##"
        mod decoy {
            pub async fn dev_login() {
                sqlx::query(
                    "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
                     WHERE discord_id = $1",
                );
            }
        }
        pub async fn dev_login() {
            sqlx::query("UPDATE users SET arma_id = $2, updated_at = now() WHERE discord_id = $1");
        }
    "##;
    let code = strip_rust_comments_outside_literals(nested);
    let first = code
        .find("fn dev_login")
        .expect("fixture sanity: decoy present");
    let live = code
        .rfind("fn dev_login")
        .expect("fixture sanity: live present");
    assert!(first < live, "fixture sanity: the decoy is textually first");
    // `code[first..live]` is exactly what the pre-T-571 `str::find` rule handed the scanner.
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(&code[first..live])),
        "fixture sanity: the first-match body is the decoy's and carries the COALESCE UPDATE \
         (this is the measured W67 hollow green)"
    );
    let body = rust_fn_body(&code, "dev_login");
    assert!(
        body.contains("SET arma_id = $2"),
        "T-571: rust_fn_body must bind the FILE-SCOPE item, not the nested decoy; got:\n{body}"
    );
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(body)),
        "T-571: nested `mod decoy {{ fn dev_login }}` + live `SET arma_id = $2` must go RED"
    );

    // ── T-571 attack 2 (named): PostgreSQL dollar-quoting hides the needle in a SELECT ─
    let dollar = r##"
        pub async fn dev_login() {
            sqlx::query(
                "SELECT 1 WHERE $decoy$UPDATE users SET arma_id = COALESCE(arma_id, $2)$decoy$ <> 'x'",
            );
        }
    "##;
    let code = strip_rust_comments_outside_literals(dollar);
    let payloads = sqlx_query_string_payloads(rust_fn_body(&code, "dev_login"));
    assert!(
        payloads
            .iter()
            .any(|p| p.contains("UPDATE users SET arma_id = COALESCE(arma_id")),
        "fixture sanity: the raw payload still embeds the needle (only the blanker removes it)"
    );
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&payloads),
        "T-571: a `$tag$…$tag$` needle inside a SELECT must go RED; got {payloads:?}"
    );
    // `$$…$$` (empty tag) is the same literal syntax and must behave the same way.
    let anon = r##"
        pub async fn dev_login() {
            sqlx::query("SELECT $$UPDATE users SET arma_id = COALESCE(arma_id, $2)$$");
        }
    "##;
    let anon_code = strip_rust_comments_outside_literals(anon);
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(rust_fn_body(
            &anon_code,
            "dev_login"
        ))),
        "T-571: `$$…$$` must be blanked exactly like `$tag$…$tag$`"
    );

    // ── No false RED: the live first-create UPDATE must still satisfy the pin ─────────
    let live_src = r#"
        pub async fn dev_login() {
            sqlx::query(
                "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
                 WHERE discord_id = $1",
            );
        }
    "#;
    let live_code = strip_rust_comments_outside_literals(live_src);
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(rust_fn_body(
            &live_code,
            "dev_login"
        ))),
        "T-571: the live COALESCE UPDATE must stay GREEN — `$1`/`$2` binds are not dollar quotes"
    );

    // Delimiter rule, stated directly: a digit cannot start a tag, so binds are safe.
    assert_eq!(pg_dollar_delim_len(b"$2), updated_at = now()", 0), None);
    assert_eq!(pg_dollar_delim_len(b"$1", 0), None);
    assert_eq!(pg_dollar_delim_len(b"$$x$$", 0), Some(2));
    assert_eq!(pg_dollar_delim_len(b"$decoy$x$decoy$", 0), Some(7));
    assert_eq!(pg_dollar_delim_len(b"$_t9$x$_t9$", 0), Some(5));
    // An unclosed delimiter is not a literal and must not swallow the rest of the payload.
    assert_eq!(pg_dollar_literal_end(b"$d$abc", 0, 3), None);
    assert_eq!(pg_dollar_literal_end(b"$d$abc$d$", 0, 3), Some(9));

    // ── The lifetime bug this module still carried (T-569 fixed only its sibling copy) ─
    // Pre-fix, the `'` of `&'static str` opened a char span that ran to the next `'` in the
    // file, so every comment in between survived "comment-stripped" source and brace depth
    // over that span was fiction — which the depth-0 rule above depends on.
    let lifetimes = "fn f() -> &'static str { /* SWALLOWED */ \"kept\" }\n// GONE\n";
    let stripped = strip_rust_comments_outside_literals(lifetimes);
    assert!(
        !stripped.contains("SWALLOWED") && !stripped.contains("GONE"),
        "T-571: a lifetime must not open a char span; got:\n{stripped}"
    );
    assert!(
        stripped.contains("\"kept\""),
        "T-571: real string literals must survive the strip; got:\n{stripped}"
    );
    // A `//` inside a raw string is text, not a comment.
    let raw = "let s = r#\"a // not a comment\"#;\n";
    assert!(
        strip_rust_comments_outside_literals(raw).contains("not a comment"),
        "T-571: raw-string contents must survive comment strip"
    );
}

/// Split a SQL column/value list on top-level commas (parens + quotes aware).
fn split_sql_list(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_quote: Option<char> = None;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = in_quote {
            cur.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
                continue;
            }
            if c == q {
                in_quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                in_quote = Some(c);
                cur.push(c);
            }
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
    out
}

/// Locate `INSERT INTO users (…) VALUES (…)` in comment-stripped handler source and
/// return the `arma_id` VALUES expression (product must be `NULL`).
fn users_insert_arma_id_value(code: &str) -> Option<String> {
    // Collapse Rust string line-continuations so the SQL reads as one line.
    let flat: String = code
        .replace("\\\r\n", " ")
        .replace("\\\n", " ")
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let insert_key = "INSERT INTO users";
    let start = flat.find(insert_key)?;
    let after_insert = &flat[start + insert_key.len()..];
    let cols_open = after_insert.find('(')?;
    let cols_close = {
        let mut depth = 0i32;
        let bytes: Vec<char> = after_insert.chars().collect();
        let mut close = None;
        for (i, &c) in bytes.iter().enumerate().skip(cols_open) {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        close?
    };
    let cols = split_sql_list(&after_insert[cols_open + 1..cols_close]);
    let arma_idx = cols.iter().position(|c| c == "arma_id")?;
    let rest = &after_insert[cols_close + 1..];
    let values_key = "VALUES";
    let vpos = rest.find(values_key)?;
    let after_values = &rest[vpos + values_key.len()..];
    let v_open = after_values.find('(')?;
    let v_close = {
        let bytes: Vec<char> = after_values.chars().collect();
        let mut depth = 0i32;
        let mut close = None;
        let mut in_quote: Option<char> = None;
        let mut i = v_open;
        while i < bytes.len() {
            let c = bytes[i];
            if let Some(q) = in_quote {
                if c == '\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if c == q {
                    in_quote = None;
                }
                i += 1;
                continue;
            }
            match c {
                '\'' | '"' => in_quote = Some(c),
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        close?
    };
    let vals = split_sql_list(&after_values[v_open + 1..v_close]);
    vals.get(arma_idx).cloned()
}

/// Class-R (T-534 / T-557 / T-560 / T-562 / T-565 / T-568): prime literals match handler,
/// and the handler keeps the race-free contract (NULL INSERT + live
/// `UPDATE users SET arma_id = COALESCE(arma_id` first-create via `sqlx::query(` SQL
/// **inside `dev_login`** — not a Rust comment / string / `format!` decoy, not a SELECT
/// with the needle only in a SQL `--` / `/* */` comment or string literal, and not a
/// dead sibling helper retaining the UPDATE while the executed path uses `$2`).
///
/// The prime seeds the same discord_id / arma_id the handler uses. If either literal
/// drifts, this goes red. T-557 moved the stamp out of INSERT; T-560 closes comment +
/// bind-INSERT hollow greps; T-562 closes string / `format!` decoys; T-565 closes
/// unrelated `sqlx::query("SELECT … -- SET arma_id = COALESCE…")` payloads; T-568 closes
/// dead-helper COALESCE + SQL string-literal needles that kept T-565 green while the
/// live UPDATE was gone (both still raced `idx_users_arma_id`).
///
/// # T-571 — what this pin is, and what it is NOT
///
/// Six generations of this pin have now been walked around (T-560 → T-562 → T-565 → T-568
/// → T-571), and every walk-around was one of two kinds:
///
/// * **Lexical** — the scanner was wrong about its input. A comment, a Rust string, a SQL
///   `--`, a `'…'` literal, a *nested* `fn dev_login`, a `$tag$…$tag$` literal. Each of
///   those is a bug with a correct answer, and each is fixed
///   ([`rust_fn_body`], [`blank_sql_string_literal_contents`], [`rust_literal_end`],
///   [`t571_pin_binds_file_scope_dev_login_and_blanks_pg_dollar_quotes`]).
/// * **Reachability** — the scanner read the right text and the text does not run. **No
///   amount of scanning fixes this one**, and this pin does not pretend to. Deciding
///   whether a call site executes, from its source, is the halting problem in a costume.
///
/// **Shapes this pin still admits — measured, not guessed** (each keeps it GREEN while the
/// first-create UPDATE never executes):
///
/// 1. `#[cfg(any())]` on the live `sqlx::query(…)` statement inside `dev_login`.
/// 2. Any other never-true `cfg` on it — `#[cfg(all(unix, any()))]`,
///    `#[cfg(feature = "nope")]`. (A literal-`#[cfg(any())]` blocklist is beaten by the
///    whitespace alone; do not add one.)
/// 3. `if false { … }` / `const NEVER: bool = false; if NEVER { … }` /
///    `if std::hint::black_box(false) { … }` around it.
/// 4. An early `return` / `?` above it.
/// 5. A macro, `include!`, or a shadowed `sqlx` module that expands to something else.
///
/// **The authority is a runtime test, not this one.**
/// `tests/misc_integration.rs::t571_dev_login_first_create_coalesces_arma_id` drives
/// `GET /auth/dev-login` against a real database and asserts the COALESCE *semantics* on
/// the row: a NULL `arma_id` is stamped, and an already-linked one survives. Dead code
/// stamps nothing, so all five shapes above fail there by construction. Keep this pin as
/// the fast, readable first failure that names the literals — not as the guarantee.
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

    let code = strip_rust_comments_outside_literals(&src);

    // T-560 / T-562 / T-565 / T-568: require the first-create UPDATE shape inside a direct
    // `sqlx::query("…")` string arg **in `dev_login`** (after SQL-comment + string-literal
    // strip). Rust comment-only (T-560), string/`format!` decoys (T-562), SELECT +
    // SQL-comment decoys (T-565), dead helpers / SQL string-literal needles (T-568) must
    // go red.
    let login_body = rust_fn_body(&code, "dev_login");
    let query_sql = sqlx_query_string_payloads(login_body);
    assert!(
        sqlx_queries_have_arma_coalesce_update(&query_sql),
        "T-560/T-562/T-565/T-568: expected live `UPDATE users SET arma_id = COALESCE(arma_id` \
         inside a direct sqlx::query(\"…\") string in comment-stripped `dev_login` — \
         a Rust comment, `let _decoy = \"…\"`, format!(\"…\"), SELECT with the needle only \
         in a SQL `--`/`/* */` comment or string literal, or a dead sibling helper retaining \
         the UPDATE while the executed path uses `$2`, is not the first-create path; without \
         the live UPDATE, concurrent cold inserts can 23505 on idx_users_arma_id again \
         (payloads: {query_sql:?})"
    );

    // T-560: INSERT must leave arma_id NULL (not a `$N` bind stamp, not a quoted literal).
    // Scoped to `dev_login` so a dead helper INSERT cannot false-green the pin.
    let arma_val = users_insert_arma_id_value(login_body).unwrap_or_else(|| {
        panic!(
            "T-560: could not parse INSERT INTO users (…) VALUES (…) arma_id slot in \
             comment-stripped `dev_login`"
        )
    });
    assert!(
        arma_val.eq_ignore_ascii_case("NULL"),
        "T-560: users INSERT must put NULL in the arma_id column (got `{arma_val}`). \
         Bind-stamping `$N` or a fixed literal into INSERT VALUES races idx_users_arma_id \
         on concurrent cold first-use — keep NULL + live COALESCE UPDATE"
    );

    // Keep the pre-T-557 quoted-literal ban (raw source) as defense-in-depth.
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

/// Class-R (T-542 / T-558): only [`require_test_database_url`] may read `TEST_DATABASE_URL`.
///
/// Scans every top-level `tests/*.rs` binary (not this `common/` module) **and** every
/// `src/**/*.rs` file (T-558). Pre-T-558 the scan was tests-only, so the in-crate
/// `#[tokio::test]` in `src/services/registry_import.rs` could read the operator base raw
/// and stay invisible. A raw `env::var("TEST_DATABASE_URL")` outside `common/mod.rs` is a
/// regression — parallel IT against live `tbd_reforger` must panic, not mutate.
#[test]
fn t542_no_raw_test_database_url_reads_outside_common() {
    let needle = concat!("env::var(", "\"TEST_DATABASE_URL\")");
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();

    // Top-level IT binaries — `tests/common/` is not scanned.
    let tests_dir = manifest.join("tests");
    let entries = std::fs::read_dir(&tests_dir)
        .unwrap_or_else(|e| panic!("T-542 Class-R: read_dir({}): {e}", tests_dir.display()));
    for entry in entries {
        let entry = entry.expect("T-542 Class-R: DirEntry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") || !path.is_file() {
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("T-542 Class-R: read {}: {e}", path.display()));
        if src.contains(needle) {
            offenders.push(format!(
                "tests/{}",
                path.file_name().unwrap().to_string_lossy()
            ));
        }
    }

    // T-558: walk `src/` so lib-target DB tests cannot hide from this pin.
    let src_dir = manifest.join("src");
    fn walk_rs(
        dir: &std::path::Path,
        needle: &str,
        offenders: &mut Vec<String>,
        root: &std::path::Path,
    ) {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("T-542/T-558 Class-R: read_dir({}): {e}", dir.display()));
        for entry in entries {
            let entry = entry.expect("T-542/T-558 Class-R: DirEntry");
            let path = entry.path();
            if path.is_dir() {
                walk_rs(&path, needle, offenders, root);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("T-542/T-558 Class-R: read {}: {e}", path.display()));
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
        "T-542/T-558: these paths still contain {needle} — use common::require_test_database_url \
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
