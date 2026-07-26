//! **T-405 — the backfill that retires the stored `javascript:` payloads, and the exact shape of
//! the SQL-vs-Rust disagreement.**
//!
//! T-391 closed the write boundary; it could do nothing about rows already in the table, and
//! `frontend/src/deployments.rs` binds `matches.aar_replay_url` into an `<a href>`. Migration
//! `0010_backfill_aar_replay_url_scheme.sql` quarantines and NULLs the offenders.
//!
//! Two things are proven here, and the first is the reason this file exists rather than a comment:
//!
//!   1. **The divergence is PINNED, not asserted.** `public.looks_like_http_url` is a regex, and
//!      `services::text::is_http_url` is a WHATWG parser; they cannot be identical, so the
//!      migration's header enumerates exactly where they part company. An enumeration in a comment
//!      rots. This runs BOTH predicates over the SAME shared case table
//!      (`apps/website/shared/is_http_url_cases.rs` — the one the two Rust implementations are
//!      already pinned to) and fails if the disagreement set is anything other than the documented
//!      one. A new divergence in either direction names itself.
//!   2. **The migration actually moves rows.** Planted rows, the real migration file executed
//!      verbatim via `include_str!`, then the table re-read. Executing the file rather than a
//!      re-typed copy of its statements is deliberate: a test that runs its own paraphrase of a
//!      migration proves the paraphrase, which is the one thing nobody ships.
//!
//! Running the file a second time is also the idempotency check, and it is free — that is what
//! `ON CONFLICT DO NOTHING` plus a self-clearing `WHERE` buys.
//!
//! Skips without `TEST_DATABASE_URL` — and a skip is a **failure to have tested**, not a pass.

use sqlx::{PgPool, Row};
use uuid::Uuid;
use website_api::db;
use website_api::services::text::is_http_url;

// The same table the two Rust implementations are pinned to. Reused here so the SQL predicate is
// held to the identical corpus rather than a friendlier one somebody wrote for it.
include!("../../shared/is_http_url_cases.rs");

/// The migration, executed verbatim rather than paraphrased. If someone edits the file, this test
/// runs the edit.
const MIGRATION: &str = include_str!("../migrations/0010_backfill_aar_replay_url_scheme.sql");

/// The complete, deliberate disagreement between `looks_like_http_url` (SQL) and `is_http_url`
/// (Rust) over the shared corpus — every input the SQL keeps and the Rust guard refuses.
///
/// Both are `http`-scheme URLs with an authority the WHATWG parser rejects as an empty host
/// (`Err(EmptyHost)`) but the regex accepts, because the regex asks only "is there a non-delimiter
/// character after the slash run" and `@` is one. Neither can execute anything: the scheme is the
/// only part that can, and on the scheme the two predicates are exactly equivalent.
///
/// **This list must never gain an entry in the other direction.** SQL rejecting something Rust
/// accepts would mean the backfill NULLs a legitimate replay link, and the test below checks for
/// that separately and loudly.
const SQL_KEEPS_RUST_REJECTS: &[&str] = &["http://@", "https://a@"];

/// Cases that cannot be stored in a Postgres `text` column at all, so the SQL predicate can never
/// be asked about them. Postgres rejects a NUL at parse time; the Rust guard still refuses these
/// because it inspects values *before* they reach the database. Skipped for that reason, and the
/// impossibility is itself asserted below rather than taken on trust.
fn unstorable(s: &str) -> bool {
    s.contains('\0')
}

async fn boot() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

/// One test rather than several, on purpose: every part of this shares one database table, and
/// `cargo test` runs a file's tests on parallel threads. Splitting it would buy nicer names and a
/// race on `matches`.
#[tokio::test]
async fn backfill_matches_the_rust_guard_and_actually_moves_rows() {
    let Some(pool) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // ── 1. Postgres genuinely cannot hold a NUL ──────────────────────────────────────────────
    // The migration header claims this is why three shared cases are unreachable. Claims about
    // the database get checked against the database.
    let nul_attempt = sqlx::query("SELECT $1::text")
        .bind("https://a.com/\u{0}x")
        .fetch_one(&pool)
        .await;
    assert!(
        nul_attempt.is_err(),
        "a NUL round-tripped through Postgres `text` — the migration's reasoning about \
         unreachable cases is wrong and its divergence list needs revisiting"
    );

    // ── 2. The SQL predicate vs the Rust guard, over the shared corpus ───────────────────────
    let mut sql_keeps_rust_rejects: Vec<&str> = Vec::new();
    let mut sql_rejects_rust_keeps: Vec<&str> = Vec::new();
    let mut checked = 0usize;

    for (input, _) in IS_HTTP_URL_CASES {
        if unstorable(input) {
            continue;
        }
        checked += 1;
        let sql_says: bool = sqlx::query_scalar("SELECT public.looks_like_http_url($1)")
            .bind(input)
            .fetch_one(&pool)
            .await
            .expect("looks_like_http_url — is migration 0010 applied?");
        let rust_says = is_http_url(input);
        match (sql_says, rust_says) {
            (true, false) => sql_keeps_rust_rejects.push(input),
            (false, true) => sql_rejects_rust_keeps.push(input),
            _ => {}
        }
    }

    assert!(
        checked >= 80,
        "only {checked} storable cases reached the SQL predicate; the shared corpus has shrunk"
    );

    // The direction that would DESTROY DATA. There is no acceptable entry here: a row the Rust
    // guard would have accepted is a legitimate replay link, and NULLing it is the one failure
    // mode this migration was told to avoid.
    assert!(
        sql_rejects_rust_keeps.is_empty(),
        "migration 0010 would NULL {} value(s) that `is_http_url` ACCEPTS — this destroys \
         legitimate replay links and the migration must not ship in this state:\n  {}",
        sql_rejects_rust_keeps.len(),
        sql_rejects_rust_keeps.join("\n  ")
    );

    // The direction that is merely lenient. Pinned exactly, so a widening goes red rather than
    // quietly making the backfill weaker than the guard it is supposed to mirror.
    assert_eq!(
        sql_keeps_rust_rejects, SQL_KEEPS_RUST_REJECTS,
        "the SQL/Rust divergence set changed. The migration header enumerates it and the two \
         must agree — update BOTH, having first checked the new entries cannot execute."
    );

    // ── 3. The backfill moves the rows it should and leaves the rest alone ───────────────────
    let tag = format!("t405-{}", Uuid::new_v4());
    sqlx::query("DELETE FROM matches WHERE source_match_id LIKE 't405-%'")
        .execute(&pool)
        .await
        .expect("clear prior run");

    // One row per storable case, plus the two sentinels the backfill must not touch.
    // `source_match_id` is uniquely indexed, so every row gets its own `<tag>-<n>`; the shared
    // `tag` prefix is what the cleanup and the counting queries match on.
    let mut planted: Vec<(Uuid, Option<&str>, bool)> = Vec::new(); // (id, value, expect_kept)
    for (n, (input, _)) in IS_HTTP_URL_CASES.iter().enumerate() {
        if unstorable(input) {
            continue;
        }
        let smid = format!("{tag}-{n}");
        let sql_says: bool = sqlx::query_scalar("SELECT public.looks_like_http_url($1)")
            .bind(input)
            .fetch_one(&pool)
            .await
            .unwrap();
        // `''` is the "no replay uploaded yet" sentinel and is excluded by the WHERE clause, so it
        // survives regardless of what the predicate says about it.
        let expect_kept = input.is_empty() || sql_says;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO matches (source_match_id, started_at, outcome, aar_replay_url) \
             VALUES ($1, now(), 'pending', $2) RETURNING id",
        )
        .bind(&smid)
        .bind(input)
        .fetch_one(&pool)
        .await
        .expect("plant row");
        planted.push((id, Some(*input), expect_kept));
    }
    // A row that is already NULL must stay NULL and must not be quarantined.
    let null_id: Uuid = sqlx::query_scalar(
        "INSERT INTO matches (source_match_id, started_at, outcome, aar_replay_url) \
         VALUES ($1, now(), 'pending', NULL) RETURNING id",
    )
    .bind(format!("{tag}-null"))
    .fetch_one(&pool)
    .await
    .expect("plant null row");
    planted.push((null_id, None, true));

    let bad_count = planted.iter().filter(|(_, _, kept)| !kept).count();
    assert!(
        bad_count >= 40,
        "only {bad_count} planted rows are expected to be scrubbed; the corpus is too weak to \
         prove this migration does anything"
    );

    // Run the REAL migration file.
    sqlx::raw_sql(MIGRATION)
        .execute(&pool)
        .await
        .expect("run migration 0010");

    let mut wrong = Vec::new();
    for (id, original, expect_kept) in &planted {
        let row = sqlx::query("SELECT aar_replay_url FROM matches WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("re-read");
        let now: Option<String> = row.get(0);
        let quarantined: Option<String> = sqlx::query_scalar(
            "SELECT original_value FROM url_quarantine \
             WHERE table_name = 'matches' AND column_name = 'aar_replay_url' AND row_id = $1",
        )
        .bind(id)
        .fetch_optional(&pool)
        .await
        .expect("read quarantine");

        if *expect_kept {
            if now.as_deref() != *original {
                wrong.push(format!(
                    "  KEPT-ROW ALTERED {original:?} -> {now:?} (the backfill destroyed a value \
                     the guard accepts)"
                ));
            }
            if quarantined.is_some() {
                wrong.push(format!("  KEPT-ROW QUARANTINED {original:?}"));
            }
        } else {
            if now.is_some() {
                wrong.push(format!(
                    "  STILL LIVE {original:?} -> {now:?} (a stored payload survived the backfill)"
                ));
            }
            if quarantined.as_deref() != *original {
                wrong.push(format!(
                    "  NOT RECOVERABLE {original:?}: quarantine holds {quarantined:?} — the value \
                     was destroyed rather than quarantined"
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "migration 0010 got {} of {} planted rows wrong:\n{}",
        wrong.len(),
        planted.len(),
        wrong.join("\n")
    );

    // ── 4. Idempotent: a second run changes nothing and duplicates nothing ───────────────────
    let quarantine_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM url_quarantine WHERE table_name = 'matches' \
         AND row_id IN (SELECT id FROM matches WHERE source_match_id LIKE $1)",
    )
    .bind(format!("{tag}-%"))
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION)
        .execute(&pool)
        .await
        .expect("re-run migration 0010");

    let quarantine_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM url_quarantine WHERE table_name = 'matches' \
         AND row_id IN (SELECT id FROM matches WHERE source_match_id LIKE $1)",
    )
    .bind(format!("{tag}-%"))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        quarantine_before, quarantine_after,
        "re-running the migration duplicated quarantine rows — it is not idempotent"
    );
    assert_eq!(
        quarantine_before as usize, bad_count,
        "quarantine holds {quarantine_before} rows but {bad_count} were scrubbed — the copy and \
         the NULL-out disagree, so some payload was destroyed without being captured"
    );

    // And a second run over a table with nothing left to fix must still be a clean no-op, which is
    // the "safe on a DB with zero bad rows" claim in the migration header.
    let still_live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM matches WHERE source_match_id LIKE $1 AND aar_replay_url IS NOT NULL \
         AND aar_replay_url <> '' AND NOT public.looks_like_http_url(aar_replay_url)",
    )
    .bind(format!("{tag}-%"))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        still_live, 0,
        "the backfill left {still_live} bad rows behind"
    );

    sqlx::query("DELETE FROM matches WHERE source_match_id LIKE $1")
        .bind(format!("{tag}-%"))
        .execute(&pool)
        .await
        .expect("cleanup");
}
