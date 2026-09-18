//! Self-checks for `tests/common/` — the integration suites' shared support module.
//!
//! These pins are pure: they derive database names, run the source-text scanners over
//! hand-written fixtures, and read the dev-login handler off disk. None of them needs a
//! database, and none of them depends on which suite is running.
//!
//! They live in their own binary because a `#[test]` written inside `tests/common/` compiles
//! into **every** suite that declares `mod common;` and therefore executes once per suite
//! binary — dozens of identical runs of the same pure assertions for one signal. Here they
//! run exactly once, and a failure names this file instead of whichever suite happened to be
//! running.

mod common;

use common::database::{
    assert_no_raw_test_database_url_reads_outside_common, database_name_from_url,
    is_safe_test_database_name, per_binary_database_name, with_database_name,
};
use common::http::{DEV_LOGIN_ARMA_ID, DEV_LOGIN_USER};
use common::source_text::{
    flatten_sql_ws, pg_dollar_delim_len, pg_dollar_literal_end, rust_fn_body,
    sqlx_queries_have_arma_coalesce_update, sqlx_query_string_payloads,
    strip_rust_comments_outside_literals, strip_sql_comments_outside_literals,
    users_insert_arma_id_value,
};

/// The derived database name is per-binary, stable, and allow-listed.
#[test]
fn per_binary_database_name_is_derived_from_the_binary() {
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
    // Every base shape in the allow-list derives to a name the allow-list still takes.
    for base in [
        "rust_it",
        "tbd_gate_it",
        "tbd_gate_w60",
        "tbd_wave6_cold",
        "tbd_scratch_probe",
    ] {
        let derived = per_binary_database_name(base, "admin_field");
        assert!(
            is_safe_test_database_name(&derived),
            "derived `{derived}` must stay inside the allow-list"
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

/// Only a direct `sqlx::query("…")` string argument counts as SQL the crate sends.
#[test]
fn sqlx_query_payloads_ignore_string_and_format_decoys() {
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
        "string/format! decoys must not count as live COALESCE; got {hollow:?}"
    );

    // A format!-wrapped query arg is not a direct string literal to sqlx::query(
    let wrapped = r#"sqlx::query(&format!("SET arma_id = COALESCE(arma_id, {})", "x"));"#;
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(wrapped)),
        "format!-wrapped sqlx::query must not satisfy the pin"
    );
}

/// The needle must be in an UPDATE statement, not in a SQL comment inside a SELECT.
#[test]
fn sqlx_query_coalesce_requires_update_not_select_comment() {
    // Grepping any sqlx::query payload for the needle stays green after the real UPDATE is
    // deleted, as long as a SELECT carries the needle in a SQL `--` comment.
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
        "SELECT + SQL-comment decoy must not count as first-create UPDATE; got {hollow:?}"
    );

    let block_comment =
        r#"sqlx::query("SELECT 1 /* UPDATE users SET arma_id = COALESCE(arma_id, $2) */");"#;
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(block_comment)),
        "SELECT + /* */ UPDATE decoy must not satisfy the pin"
    );

    let live = r#"
        sqlx::query(
            "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
             WHERE discord_id = $1",
        );
    "#;
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(live)),
        "the real first-create UPDATE must still satisfy the pin"
    );
}

/// The pin binds to `dev_login`'s body, and SQL string-literal contents are not statements.
#[test]
fn coalesce_pin_rejects_dead_helper_and_sql_string_literal() {
    // Attack 1: COALESCE parked in an unused sibling fn while the live first-create path uses
    // `SET arma_id = $2`. A whole-file payload scan stays green; the pin must bind to
    // `dev_login` only.
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
    // Sanity: a whole-file scan still sees the dead helper (the hollow shape being closed).
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(&code)),
        "fixture sanity: dead helper still embeds a COALESCE UPDATE payload at file scope"
    );
    let login_payloads = sqlx_query_string_payloads(rust_fn_body(&code, "dev_login"));
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&login_payloads),
        "dead-helper COALESCE + live `SET arma_id = $2` must go RED when pinned to \
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
    // Comment-strip alone would still match this one — only the literal blanker removes it.
    assert!(
        lit_payloads.iter().any(|p| {
            let flat = flatten_sql_ws(&strip_sql_comments_outside_literals(p));
            flat.contains("UPDATE users SET arma_id = COALESCE(arma_id")
        }),
        "fixture sanity: comment-strip alone still sees the string-literal needle"
    );
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&lit_payloads),
        "SELECT with COALESCE needle inside a SQL string literal must go RED; \
         got {lit_payloads:?}"
    );

    // The SELECT + `--` comment decoy must remain RED under the hardened pin.
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
        "the SELECT + `--` comment decoy must stay RED"
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
        "the live COALESCE UPDATE inside dev_login must still satisfy the pin"
    );
}

/// Two lexical walk-arounds the scanners must refuse: a nested `fn dev_login` that wins
/// `str::find`, and PostgreSQL dollar-quoting hiding the needle inside a SELECT.
#[test]
fn pin_binds_file_scope_dev_login_and_blanks_pg_dollar_quotes() {
    // ── A nested `fn dev_login` decoy is textually first ──────────────────────────────
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
    // `code[first..live]` is exactly what a `str::find` rule would hand the scanner.
    assert!(
        sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(&code[first..live])),
        "fixture sanity: the first-match body is the decoy's and carries the COALESCE UPDATE \
         (this is the hollow green a find-first rule produces)"
    );
    let body = rust_fn_body(&code, "dev_login");
    assert!(
        body.contains("SET arma_id = $2"),
        "rust_fn_body must bind the FILE-SCOPE item, not the nested decoy; got:\n{body}"
    );
    assert!(
        !sqlx_queries_have_arma_coalesce_update(&sqlx_query_string_payloads(body)),
        "nested `mod decoy {{ fn dev_login }}` + live `SET arma_id = $2` must go RED"
    );

    // ── PostgreSQL dollar-quoting hides the needle in a SELECT ────────────────────────
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
        "a `$tag$…$tag$` needle inside a SELECT must go RED; got {payloads:?}"
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
        "`$$…$$` must be blanked exactly like `$tag$…$tag$`"
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
        "the live COALESCE UPDATE must stay GREEN — `$1`/`$2` binds are not dollar quotes"
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

    // ── A lifetime must not open a char span ─────────────────────────────────────────
    // Treating the `'` of `&'static str` as a char literal opens a span running to the next
    // `'` in the file, so every comment in between survives "comment-stripped" source and the
    // brace depth the file-scope rule above depends on becomes fiction.
    let lifetimes = "fn f() -> &'static str { /* SWALLOWED */ \"kept\" }\n// GONE\n";
    let stripped = strip_rust_comments_outside_literals(lifetimes);
    assert!(
        !stripped.contains("SWALLOWED") && !stripped.contains("GONE"),
        "a lifetime must not open a char span; got:\n{stripped}"
    );
    assert!(
        stripped.contains("\"kept\""),
        "real string literals must survive the strip; got:\n{stripped}"
    );
    // A `//` inside a raw string is text, not a comment.
    let raw = "let s = r#\"a // not a comment\"#;\n";
    assert!(
        strip_rust_comments_outside_literals(raw).contains("not a comment"),
        "raw-string contents must survive comment strip"
    );
}

/// The dev-login prime literals still match the handler, and the handler keeps the race-free
/// first-create contract: a NULL `arma_id` in the INSERT plus a live
/// `UPDATE users SET arma_id = COALESCE(arma_id` sent from a direct `sqlx::query("…")`
/// **inside `dev_login`**.
///
/// The prime in `tests/common/database.rs` seeds the same `discord_id` / `arma_id` the
/// handler uses. If either literal drifts, this goes red.
///
/// # What this pin is, and what it is NOT
///
/// Walk-arounds of a source-text pin come in two kinds:
///
/// * **Lexical** — the scanner was wrong about its input: a Rust comment, a Rust string, a
///   SQL `--`, a `'…'` literal, a *nested* `fn dev_login`, a `$tag$…$tag$` literal. Each is a
///   defect with a correct answer, and each is handled in `tests/common/source_text.rs`.
/// * **Reachability** — the scanner read the right text and the text does not run. **No
///   amount of scanning fixes this one**, and this pin does not pretend to. Deciding whether
///   a call site executes, from its source, is the halting problem in a costume.
///
/// **Shapes this pin still admits** (each keeps it GREEN while the first-create UPDATE never
/// executes):
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
/// **The authority is a runtime test, not this one.** `tests/misc_integration.rs` drives
/// `GET /auth/dev-login` against a real database and asserts the COALESCE *semantics* on the
/// row: a NULL `arma_id` is stamped, and an already-linked one survives. Dead code stamps
/// nothing, so all five shapes above fail there by construction. Keep this pin as the fast,
/// readable first failure that names the literals — not as the guarantee.
#[test]
fn dev_login_prime_literals_still_match_handler() {
    let handler = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/identity_and_access/handlers/developer_login.rs");
    let src = std::fs::read_to_string(&handler)
        .unwrap_or_else(|e| panic!("read {}: {e}", handler.display()));
    for needle in [DEV_LOGIN_USER, DEV_LOGIN_ARMA_ID] {
        assert!(
            src.contains(needle),
            "src/identity_and_access/handlers/developer_login.rs no longer contains \
             `{needle}`. The dev-login prime in tests/common/database.rs seeds that exact \
             row; update both together."
        );
    }
    assert!(
        src.contains("ON CONFLICT (discord_id) DO UPDATE"),
        "dev_login's upsert lost ON CONFLICT (discord_id) DO UPDATE — re-derive the race \
         analysis before changing the prime"
    );

    let code = strip_rust_comments_outside_literals(&src);

    // Require the first-create UPDATE shape inside a direct `sqlx::query("…")` string arg
    // **in `dev_login`**, after SQL-comment and string-literal strip.
    let login_body = rust_fn_body(&code, "dev_login");
    let query_sql = sqlx_query_string_payloads(login_body);
    assert!(
        sqlx_queries_have_arma_coalesce_update(&query_sql),
        "expected live `UPDATE users SET arma_id = COALESCE(arma_id` inside a direct \
         sqlx::query(\"…\") string in comment-stripped `dev_login` — a Rust comment, \
         `let _decoy = \"…\"`, format!(\"…\"), a SELECT with the needle only in a SQL \
         `--`/`/* */` comment or string literal, or a dead sibling helper retaining the \
         UPDATE while the executed path uses `$2`, is not the first-create path; without the \
         live UPDATE, concurrent cold inserts can 23505 on idx_users_arma_id again \
         (payloads: {query_sql:?})"
    );

    // The INSERT must leave arma_id NULL (not a `$N` bind stamp, not a quoted literal).
    // Scoped to `dev_login` so a dead helper INSERT cannot false-green the pin.
    let arma_val = users_insert_arma_id_value(login_body).unwrap_or_else(|| {
        panic!(
            "could not parse INSERT INTO users (…) VALUES (…) arma_id slot in \
             comment-stripped `dev_login`"
        )
    });
    assert!(
        arma_val.eq_ignore_ascii_case("NULL"),
        "users INSERT must put NULL in the arma_id column (got `{arma_val}`). Bind-stamping \
         `$N` or a fixed literal into INSERT VALUES races idx_users_arma_id on concurrent \
         cold first-use — keep NULL + live COALESCE UPDATE"
    );

    // Keep the quoted-literal ban on raw source as defence in depth.
    assert!(
        !src.contains("'', 'dev-arma-76561190000000001'"),
        "a fixed arma_id must not appear as an INSERT VALUES literal after empty avatar_url \
         — that is the concurrent-first-use 23505 shape"
    );
}

/// The allow/deny table for the test-database target guard.
#[test]
fn test_database_name_guard_refuses_unsafe_names() {
    // Makefile + wave gate + operator cold.
    assert!(is_safe_test_database_name("rust_it"));
    assert!(is_safe_test_database_name("tbd_gate_it"));
    assert!(is_safe_test_database_name("tbd_gate_w54"));
    assert!(is_safe_test_database_name("tbd_gate_migrate"));
    assert!(is_safe_test_database_name("tbd_wave6_cold"));
    assert!(is_safe_test_database_name("tbd_scratch_cold"));
    assert!(is_safe_test_database_name("tbd_scratch_probe"));
    assert!(is_safe_test_database_name("tbd_scratch_it"));
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

/// Only `common::require_test_database_url` may read `TEST_DATABASE_URL`.
#[test]
fn no_raw_test_database_url_reads_outside_common() {
    assert_no_raw_test_database_url_reads_outside_common();
}
