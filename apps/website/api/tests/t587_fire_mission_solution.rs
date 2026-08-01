//! T-587 — `POST /api/v1/fire-missions` must persist the solution it computed, and
//! `GET /api/v1/events/{id}/fire-missions` must hand it back.
//!
//! # Why this suite exists in this shape
//!
//! The defect is a save endpoint that answers **201 CREATED** carrying a full firing solution and
//! writes a third of it. `charge`, `azimuth_mils` and `time_of_flight_s` were computed, serialised
//! into the response, and dropped on the way to the INSERT; the four coordinates the caller sent
//! reached no column at all. Every one of those is invisible from the response body, because the
//! response is built from the in-memory `FireSolution` and never re-reads the row. That is this
//! program's signature defect exactly — a tool reporting success over an input it never examined —
//! and it means **a test that asserts on the 201 body proves nothing here.** The pre-fix handler
//! passes that test.
//!
//! So every assertion below lands on one of two things the handler cannot fake:
//!
//! * the **database row**, read back with a direct `sqlx` query on its own pool connection, not
//!   through any handler;
//! * the **list endpoint's** body, which is built by a `SELECT` — so a column the INSERT never
//!   wrote cannot appear in it, and a column the `SELECT` forgets to project cannot either.
//!
//! # The four cases
//!
//! 1. [`saved_solution_reaches_the_row_and_comes_back_out`] — the ticket. Save, read the row,
//!    read the list, and require all three to agree on all seven values.
//! 2. [`a_row_written_before_this_migration_still_lists_and_restores`] — the columns are nullable
//!    because rows predating the migration exist. One is forged directly into the table with all
//!    seven `NULL` and must come back as `null` rather than as `0`, and must not panic the
//!    handler's decode.
//! 3. [`the_shipped_backfill_recovers_coordinates_from_the_grid_encoding`] — runs the migration's
//!    **own** `UPDATE` statements, read out of the shipped `.sql` file, over rows this test
//!    inserts. A transcribed copy of the SQL would test the copy.
//! 4. [`out_of_range_and_unknown_weapon_still_answer_422_and_400`] — T-587 deleted an unreachable
//!    guard in `solve_checked`; these are the two statuses that must not have moved with it.
//!
//! Skips without `TEST_DATABASE_URL`, like every DB-backed suite in this crate.

mod common;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

/// FP (1000, 2000) → TGT (2200, 1800) on an `M252 81mm`: the T-285 field report's own probe.
/// 1217 m at 99.5°, which `services/mortar.rs` reaches on charge 2 — a solution with a
/// **non-zero** charge and a **non-zero** TOF, so a handler that wrote zeros could not pass by
/// accident.
const SAVE_BODY: &str = r#"{"weapon_system":"M252 81mm","fp_x":1000,"fp_y":2000,"tgt_x":2200,"tgt_y":1800,"fp_grid":"1000, 2000","target_grid":"2200, 1800","event_id":"EVENT"}"#;

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "t587-secret"),
    ));
    Some((app, pool))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    tok: &str,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {tok}"));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The stored row, straight out of the table.
///
/// Deliberately **not** `models::FireMission` via `query_as`: that struct is what the handler
/// deserialises into, so sharing it would let one wrong column name agree with itself on both
/// sides. Naming the columns here means the test fails if the migration named them differently
/// from what the handler binds.
type StoredRow = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<i64>,
    Option<i64>,
    Option<f64>,
);

async fn stored_solution(pool: &PgPool, id: &str) -> StoredRow {
    sqlx::query_as(
        "SELECT fp_x, fp_y, tgt_x, tgt_y, azimuth_mils, charge, time_of_flight_s \
         FROM fire_missions WHERE id = $1::uuid",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("T-587: read the stored fire mission back")
}

/// **The ticket.** Save a solution, then prove it is in the database and comes back out.
#[tokio::test]
async fn saved_solution_reaches_the_row_and_comes_back_out() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let tok = common::dev_login_token(&app, "t587_save", "admin").await;
    let event = Uuid::new_v4().to_string();

    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/fire-missions",
        &tok,
        Some(&SAVE_BODY.replace("EVENT", &event)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "save fire: {body}");

    // ── What the handler CLAIMS it computed. Not evidence of anything being stored: this half
    // ── of the body is `serde_json::to_value(&sol)` and was identical before T-587.
    let sol = &body["solution"];
    assert_eq!(sol["distance_m"], 1217);
    assert_eq!(sol["charge"], 2);
    assert_eq!(sol["azimuth_mils"], 1768);
    assert_eq!(sol["time_of_flight_s"], 29.4);
    let id = body["fire_mission"]["id"]
        .as_str()
        .expect("201 carries the row id")
        .to_string();

    // ── What is actually in the table. This is the assertion the pre-fix handler fails: it
    // ── answered the identical 201 above with all seven of these NULL.
    let (fp_x, fp_y, tgt_x, tgt_y, az_mils, charge, tof) = stored_solution(&pool, &id).await;
    assert_eq!(
        (fp_x, fp_y, tgt_x, tgt_y),
        (Some(1000.0), Some(2000.0), Some(2200.0), Some(1800.0)),
        "the four coordinates the caller sent are not in the row"
    );
    assert_eq!(charge, Some(2), "charge is not in the row");
    assert_eq!(az_mils, Some(1768), "azimuth_mils is not in the row");
    assert_eq!(tof, Some(29.4), "time_of_flight_s is not in the row");

    // ── And the row agrees with what the caller was told. A handler that stored a *different*
    // ── charge from the one it returned would pass both blocks above and fail here.
    assert_eq!(charge.map(Value::from), Some(sol["charge"].clone()));
    assert_eq!(az_mils.map(Value::from), Some(sol["azimuth_mils"].clone()));
    assert_eq!(tof.map(Value::from), Some(sol["time_of_flight_s"].clone()));

    // ── The 201's own `fire_mission` half comes from `RETURNING`, so it must carry them too.
    let returned = &body["fire_mission"];
    assert_eq!(returned["charge"], 2, "RETURNING dropped charge");
    assert_eq!(returned["time_of_flight_s"], 29.4, "RETURNING dropped TOF");
    assert_eq!(returned["fp_x"], 1000.0, "RETURNING dropped fp_x");

    // ── The read path. Built by a SELECT, so a column the INSERT never wrote cannot show up
    // ── here, and one the SELECT forgets to project cannot either.
    let (st, list) = call(
        &app,
        "GET",
        &format!("/api/v1/events/{event}/fire-missions"),
        &tok,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "list: {list}");
    let rows = list["data"].as_array().expect("data is an array");
    assert_eq!(rows.len(), 1, "one saved fire mission on this operation");
    let row = &rows[0];
    assert_eq!(row["id"], id.as_str());
    assert_eq!(row["fp_x"], 1000.0);
    assert_eq!(row["fp_y"], 2000.0);
    assert_eq!(row["tgt_x"], 2200.0);
    assert_eq!(row["tgt_y"], 1800.0);
    assert_eq!(row["azimuth_mils"], 1768);
    assert_eq!(row["charge"], 2, "a reload cannot see the charge");
    assert_eq!(
        row["time_of_flight_s"], 29.4,
        "a reload cannot see the time of flight — the T-285 asymmetry"
    );
    // The shipped columns are untouched by this change.
    assert_eq!(row["distance_m"], 1217);
    assert_eq!(row["azimuth_deg"], 99.5);
    assert_eq!(row["elevation_mils"], sol["elevation_mils"]);
    assert_eq!(row["fp_grid"], "1000, 2000");
    assert_eq!(row["target_grid"], "2200, 1800");
}

/// A fire mission saved before this migration must still list and still read as "not recorded".
///
/// The columns are nullable precisely because rows like this exist, and the failure mode is not a
/// 500 — it is a `0` where a `null` belongs. `charge: 0` names a real ring on every tube in
/// `charges_for` and `time_of_flight_s: 0.0` is a plausible flight; either would render on the
/// calculator's card as a confident, wrong, unfalsifiable number for a mission nobody re-checked.
///
/// Forged with a direct INSERT that names only the pre-T-587 columns, which is byte-for-byte what
/// the old handler's statement did.
#[tokio::test]
async fn a_row_written_before_this_migration_still_lists_and_restores() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let tok = common::dev_login_token(&app, "t587_legacy", "admin").await;
    let event = Uuid::new_v4().to_string();

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO fire_missions \
         (event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, azimuth_deg, \
          elevation_mils, created_at) \
         VALUES ($1::uuid, '000000000000000001', 'M252 81mm', '1000, 2000', '2200, 1800', \
                 1217, 99.5, 1315, now()) \
         RETURNING id",
    )
    .bind(&event)
    .fetch_one(&pool)
    .await
    .expect("T-587: forge a pre-migration fire mission");

    // Nothing filled them in behind our back — no DEFAULT, no trigger.
    let (fp_x, fp_y, tgt_x, tgt_y, az_mils, charge, tof) =
        stored_solution(&pool, &id.to_string()).await;
    assert_eq!(
        (fp_x, fp_y, tgt_x, tgt_y, az_mils, charge, tof),
        (None, None, None, None, None, None, None),
        "a column added by T-587 acquired a default — a stored 0 claims to be a measurement"
    );

    let (st, list) = call(
        &app,
        "GET",
        &format!("/api/v1/events/{event}/fire-missions"),
        &tok,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "a pre-migration row must not break the list: {list}"
    );
    let row = &list["data"].as_array().expect("data is an array")[0];
    // `null`, explicitly present — not absent, and above all not `0`.
    for f in [
        "fp_x",
        "fp_y",
        "tgt_x",
        "tgt_y",
        "azimuth_mils",
        "charge",
        "time_of_flight_s",
    ] {
        assert_eq!(
            row[f],
            Value::Null,
            "{f} on a pre-migration row must be null"
        );
    }
    // …while everything the old schema DID hold still reads exactly as before.
    assert_eq!(row["distance_m"], 1217);
    assert_eq!(row["azimuth_deg"], 99.5);
    assert_eq!(row["elevation_mils"], 1315);
    assert_eq!(row["fp_grid"], "1000, 2000");
}

/// The migration's coordinate backfill, run from the shipped `.sql` file.
///
/// Reading the statements out of the migration rather than retyping them is the point: a
/// transcribed copy would test the transcription, and the two would drift the first time the
/// accept regex is touched. The `ALTER` half is skipped because provisioning already applied it —
/// the `UPDATE`s are idempotent over rows that are already correct, which is what lets them be
/// replayed here at all.
///
/// The criterion under test is agreement with `frontend/src/mortar.rs::parse_grid`: accept what it
/// accepts, refuse what it refuses. Accept more and the migration invents coordinates for rows the
/// calculator has always shown as unrestorable; accept less and it strands rows it has always
/// restored.
#[tokio::test]
async fn the_shipped_backfill_recovers_coordinates_from_the_grid_encoding() {
    let Some((_app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let event = Uuid::new_v4().to_string();

    // `fp_grid`, `target_grid`, and what each pair of columns must hold after the backfill.
    // `None` = "this grid is not `fmt_grid`'s encoding, so the row keeps NULL coordinates".
    type Coords = Option<(f64, f64)>;
    type BackfillCase = (&'static str, &'static str, Coords, Coords);
    let cases: [BackfillCase; 6] = [
        // What `fmt_grid` writes — whole metres, fractions, negatives.
        (
            "1000, 2000",
            "2200, 1800",
            Some((1000.0, 2000.0)),
            Some((2200.0, 1800.0)),
        ),
        (
            "2200.5, 1800.25",
            "-750, 12800",
            Some((2200.5, 1800.25)),
            Some((-750.0, 12800.0)),
        ),
        ("0, 0", "0.1, -0.1", Some((0.0, 0.0)), Some((0.1, -0.1))),
        // A six-figure military reference is not this encoding and must stay NULL — the honest
        // answer, and the same one the calculator gives that row today.
        ("012345", "012845", None, None),
        // Partial junk: neither half parses.
        ("AB, CD", "1000, ", None, None),
        // One grid is the encoding and the other is not. The two pairs are backfilled by two
        // independent statements, so this row gets one real pair and one NULL pair.
        ("500, 600", "GRID REF ALPHA", Some((500.0, 600.0)), None),
    ];

    for (fp_grid, target_grid, _, _) in cases {
        sqlx::query(
            "INSERT INTO fire_missions \
             (event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, azimuth_deg, \
              elevation_mils, created_at) \
             VALUES ($1::uuid, '000000000000000001', 'M252 81mm', $2, $3, 1, 0.0, 1000, now())",
        )
        .bind(&event)
        .bind(fp_grid)
        .bind(target_grid)
        .execute(&pool)
        .await
        .expect("T-587: insert a backfill fixture row");
    }

    // Replay the migration's own UPDATEs, scoped to this test's rows so a parallel sibling's
    // fixtures are untouched.
    const MIGRATION: &str = include_str!("../migrations/0020_fire_missions_solution.sql");
    // Comment lines go first: 0020's rationale block contains prose semicolons, and splitting
    // statements before stripping them would shred the file into fragments.
    let sql_only = MIGRATION
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    let updates: Vec<&str> = sql_only
        .split(';')
        .map(str::trim)
        .filter(|s| s.starts_with("UPDATE"))
        .collect();
    assert_eq!(
        updates.len(),
        2,
        "expected the two coordinate backfill statements in 0020; found {}",
        updates.len()
    );
    for stmt in &updates {
        // `event` is a `Uuid` this test generated, not caller input — that is the whole audit
        // `AssertSqlSafe` is asking for. The scoping exists so a parallel sibling suite's fixture
        // rows in the same database are not rewritten by this replay.
        let scoped = format!("{stmt} AND event_id = '{event}'::uuid");
        sqlx::raw_sql(sqlx::AssertSqlSafe(scoped.clone()))
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("T-587: replay the shipped backfill: {e}\n{scoped}"));
    }

    for (fp_grid, target_grid, want_fp, want_tgt) in cases {
        let got: (Option<f64>, Option<f64>, Option<f64>, Option<f64>) = sqlx::query_as(
            "SELECT fp_x, fp_y, tgt_x, tgt_y FROM fire_missions \
             WHERE event_id = $1::uuid AND fp_grid = $2 AND target_grid = $3",
        )
        .bind(&event)
        .bind(fp_grid)
        .bind(target_grid)
        .fetch_one(&pool)
        .await
        .expect("T-587: read a backfilled row");
        assert_eq!(
            (got.0, got.1),
            (want_fp.map(|p| p.0), want_fp.map(|p| p.1)),
            "fp_grid {fp_grid:?} backfilled wrong"
        );
        assert_eq!(
            (got.2, got.3),
            (want_tgt.map(|p| p.0), want_tgt.map(|p| p.1)),
            "target_grid {target_grid:?} backfilled wrong"
        );
    }
}

/// T-587 collapsed `solve_checked` onto a direct `match`, deleting a guard that had been
/// unreachable since T-365. These are the two statuses that must not have moved with it — an
/// unknown weapon is a **400** and it beats the **422** an out-of-range target gets, and the 422
/// still carries the partial solution in `details`.
#[tokio::test]
async fn out_of_range_and_unknown_weapon_still_answer_422_and_400() {
    let Some((app, _pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let tok = common::dev_login_token(&app, "t587_guards", "admin").await;

    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/fire-missions/solve",
        &tok,
        Some(r#"{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":100000}"#),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "out of range: {body}");
    assert_eq!(
        body["details"]["distance_m"], 100000,
        "the 422 must still carry the partial solution — the OutOfRange payload is on the wire"
    );

    for weapon in ["M120 120mmm", "Potato Launcher", "m252_81mm"] {
        let (st, body) = call(
            &app,
            "POST",
            "/api/v1/fire-missions/solve",
            &tok,
            Some(&format!(
                r#"{{"weapon_system":"{weapon}","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":100000}}"#
            )),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{weapon}: {body}");
        assert_eq!(
            body["error"],
            format!("unknown weapon_system '{weapon}'"),
            "the weapon must be reported verbatim"
        );
    }
}
