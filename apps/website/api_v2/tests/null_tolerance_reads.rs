//! The behavioural half: with every nullable column NULL on rows the caller can actually
//! reach, no GET route may 5xx — plus the guard that the sweep covers the whole router.
//!
//! Skips without `TEST_DATABASE_URL`.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use serde_json::Value;
use uuid::Uuid;

mod common;
mod null_tolerance_support;

use null_tolerance_support::database_fixtures::*;
use null_tolerance_support::source_scan::*;
use null_tolerance_support::*;

/// The two database tests share this suite's fixtures on one database, and `boot()` clears the
/// previous run's rows — so without this a concurrent `boot()` would delete the other test's seed
/// mid-sweep. `cargo test` runs test fns in parallel by default, so the serialisation has to be
/// in the code, not in a `--test-threads=1` someone has to remember.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// The behavioural regression: with **every** nullable column NULL on rows the caller can
/// actually reach, no GET route may 5xx.
///
/// The canonical shape — a handler selecting a bare `orbat_slots.*` — fails here as
/// `500 … error occurred while decoding column "tag": unexpected null`.
#[tokio::test]
async fn every_nullable_column_null_and_every_get_route_still_serves() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool, tok)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let s = seed(&pool).await;
    let nullable = nullable_columns(&pool).await;

    let mut blasted = 0usize;
    for (table, where_sql) in &s.rows {
        blasted += blast_nulls(&pool, table, where_sql, &nullable).await.len();
    }
    // The suite's own reach: if this collapses, the sweep below has stopped proving anything.
    assert!(
        blasted >= 80,
        "only {blasted} nullable columns were NULLed; the schema has \
         {} across {} tables — the seed has drifted away from the schema",
        nullable.values().map(BTreeSet::len).sum::<usize>(),
        nullable.len()
    );

    let mut failed: Vec<(&'static str, String)> = Vec::new();
    for (template, uri, caller) in route_sweep(&s) {
        let (st, body) = get(&app, &uri, &tok, &s.machine_secret, caller).await;
        if st.is_server_error() {
            failed.push((
                template,
                format!("{template} → {uri}: {st} {}", body.trim()),
            ));
        }
    }

    let healed: Vec<&str> = KNOWN_OPEN_ROUTES
        .iter()
        .filter(|(r, ..)| !failed.iter().any(|(t, _)| t == r))
        .map(|(r, ..)| *r)
        .collect();
    if !healed.is_empty() {
        eprintln!(
            "note: KNOWN_OPEN_ROUTES entries now survive the NULL blast — the defect is \
             fixed, so prune them from tests/null_tolerance_support/mod.rs: {healed:?}"
        );
    }

    let unexpected: Vec<&String> = failed
        .iter()
        .filter(|(t, _)| !KNOWN_OPEN_ROUTES.iter().any(|(r, ..)| r == t))
        .map(|(_, msg)| msg)
        .collect();
    assert!(
        unexpected.is_empty(),
        "a nullable column decoded into a non-Option field. COALESCE it in the query — do NOT \
         make the model field Option (see match_telemetry::models::match_record::Match).\n  {}",
        unexpected
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// Approvals `submitted_at`, plus the structural pin on the mission timestamp columns.
///
/// Migration 0015 makes `missions.created_at` / `updated_at` `DEFAULT now() NOT NULL`, so an
/// INSERT carrying an explicit NULL fails with Postgres **23502**. That is what makes the
/// second and third links of the `COALESCE(updated_at, created_at, sentinel)` chain
/// unreachable through the API at all.
///
/// Two halves, both load-bearing:
///   1. **Structural pin** — planting NULL into either timestamp column must fail
///      23502. Reverting 0015's NOT NULL fails here loudly.
///   2. **Minimal behavioural assert** — with both timestamps set, `GET /approvals` must
///      report `updated_at` as `submitted_at` (the first COALESCE arm). The remaining arms are
///      unreachable via INSERT, and the structural pin above owns that guarantee.
#[tokio::test]
async fn approvals_queue_reports_an_honest_submitted_at_over_null_timestamps() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool, tok)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // Migration 0015's structural guarantee: explicit NULL into either NOT NULL column → 23502.
    // Omitting the column would silently take DEFAULT now() — that would be a false green.
    let pin = "2026-03-04T05:06:07Z";
    for (label, created, updated) in [
        ("both NULL", None, None),
        ("created_at NULL", None, Some(pin)),
        ("updated_at NULL", Some(pin), None),
    ] {
        let id = Uuid::new_v4();
        let err = sqlx::query(
            "INSERT INTO missions (id, title, author_id, terrain, game_mode, weather, time_of_day, \
             max_players, status, created_at, updated_at) \
             VALUES ($1, 'Null Approval 23502', $2, 'everon', 'pve_coop', 'clear', '14:00', 16, \
             'pending_approval', $3::timestamptz, $4::timestamptz)",
        )
        .bind(id)
        .bind(NULL_UID)
        .bind(created)
        .bind(updated)
        .execute(&pool)
        .await
        .expect_err(&format!(
            "INSERT with {label} must be rejected (missions.created_at/updated_at \
             are DEFAULT now() NOT NULL after migration 0015)"
        ));
        let db = err
            .as_database_error()
            .unwrap_or_else(|| panic!("expected a database error for {label}, got: {err}"));
        assert_eq!(
            db.code().as_deref(),
            Some("23502"),
            "{label} must fail not-null violation 23502, got code {:?} — {err}",
            db.code()
        );
    }

    // Behavioural: both timestamps real — COALESCE prefers updated_at.
    let pending = Uuid::new_v4();
    let created = "2026-01-02T03:04:05Z";
    let updated = "2026-03-04T05:06:07Z";
    sqlx::query(
        "INSERT INTO missions (id, title, author_id, terrain, game_mode, weather, time_of_day, \
         max_players, status, created_at, updated_at) \
         VALUES ($1, 'Null Approval Live', $2, 'everon', 'pve_coop', 'clear', '14:00', 16, \
         'pending_approval', $3::timestamptz, $4::timestamptz)",
    )
    .bind(pending)
    .bind(NULL_UID)
    .bind(created)
    .bind(updated)
    .execute(&pool)
    .await
    .expect("seed pending_approval mission with real timestamps");

    let (st, body) = get(&app, "/api/v1/approvals", &tok, "", SweepCaller::Member).await;
    assert_eq!(st, StatusCode::OK, "approvals: {body}");
    let v: Value = serde_json::from_str(&body).expect("approvals json");
    let row = v["data"]
        .as_array()
        .expect("approvals data array")
        .iter()
        .find(|r| r["mission_id"] == pending.to_string())
        .unwrap_or_else(|| panic!("mission {pending} missing from the approvals queue"));
    assert_eq!(
        row["submitted_at"], updated,
        "with both timestamps set, submitted_at must be updated_at (first COALESCE arm) — \
         got {}",
        row["submitted_at"]
    );
}

/// Guards failure mode 1: the sweep must cover the whole router, not a remembered subset.
///
/// Parses the registered GET routes out of the api_v2 route tables and fails when one is neither swept nor
/// explicitly skipped with a reason — so adding a route that reads a nullable column cannot
/// silently escape this file. No database needed.
#[test]
fn every_get_route_is_swept_or_skipped_with_a_reason() {
    // Source pins: no instance of this defect is open, so both the tolerance list and its
    // ceiling stay at zero. Re-adding an entry *or* bumping the cap must RED. With BASELINE_CAP
    // pinned at 0, `baseline <= BASELINE_CAP` is identical to `baseline == 0` (and
    // clippy::absurd_extreme_comparisons denies the `<=` form).
    assert_eq!(
        BASELINE_CAP, 0,
        "BASELINE_CAP must remain 0 — raising it re-opens silent tolerance slack"
    );
    let baseline = KNOWN_OPEN.len() + KNOWN_OPEN_ROUTES.len();
    assert_eq!(
        baseline, BASELINE_CAP,
        "KNOWN_OPEN + KNOWN_OPEN_ROUTES hold {baseline} entries, over BASELINE_CAP of \
         {BASELINE_CAP}. Fix the defect, or raise the cap deliberately so a reviewer sees it."
    );

    let src = router_source();
    let registered = registered_get_routes(&src);
    assert!(
        registered.len() > 40,
        "parsed only {} GET routes out of the eight domain route tables and src/core/http_router.rs \
         — the parser has drifted from the source and this guard is no longer guarding anything",
        registered.len()
    );

    // A dummy seed is enough: only the templates are read here.
    let dummy = Seed {
        mission: Uuid::nil(),
        pending_mission: Uuid::nil(),
        version: Uuid::nil(),
        event: Uuid::nil(),
        event_mission: Uuid::nil(),
        announcement: Uuid::nil(),
        server: Uuid::nil(),
        machine_secret: String::new(),
        command: Uuid::nil(),
        artifact: Uuid::nil(),
        deployment: Uuid::nil(),
        faction: Uuid::nil(),
        wiki_slug: String::new(),
        rows: Vec::new(),
    };
    let swept: BTreeSet<&str> = route_sweep(&dummy).into_iter().map(|(t, _, _)| t).collect();
    let skipped: BTreeSet<&str> = ROUTE_SWEEP_SKIP.iter().map(|(r, _)| *r).collect();

    let missing: Vec<&String> = registered
        .iter()
        .filter(|r| !swept.contains(r.as_str()) && !skipped.contains(r.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "GET routes registered by the api_v2 route tables but neither swept by route_sweep() nor \
         listed in ROUTE_SWEEP_SKIP with a reason: {missing:?}"
    );

    let stale: Vec<&&str> = swept
        .iter()
        .chain(skipped.iter())
        .filter(|r| !registered.contains(**r))
        .collect();
    assert!(
        stale.is_empty(),
        "route_sweep()/ROUTE_SWEEP_SKIP name routes that the api_v2 route tables no longer register: \
         {stale:?}"
    );
}
