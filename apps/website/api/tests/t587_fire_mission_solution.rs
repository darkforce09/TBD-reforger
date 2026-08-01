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
//! # The cases
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
//! # T-626 — the claim case 3 was not checking
//!
//! 0020's comment calls its accept regex `parse_grid`'s, "deliberately character for character".
//! It is not: `parse::<f64>` also takes `+1000, 2000`, `.5, 2`, `5., 2` and `1e3, 500`, and the
//! regex takes none of them. Case 3's original six fixtures omitted **exactly** those four forms,
//! so the suite agreed with the claim by never testing it — the same shape as the defect this file
//! was written for. Two cases close that:
//!
//! 5. [`the_backfill_regex_is_narrower_than_parse_grid`] — measures both readers on the divergent
//!    forms, on the agreed forms (same `f64` bits, not just "both accept"), and on the one input
//!    class where the regex is the *wider* of the two.
//! 6. [`the_transcription_of_parse_grid_is_still_the_shipped_one`] — case 5 needs a copy of
//!    `parse_grid` (the frontend is a wasm crate and cannot be linked here); this pins the copy
//!    against the shipped function token for token.
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
/// The criterion under test is **not** "accept exactly what `parse_grid` accepts" — 0020's comment
/// claims that and it is false (T-626). It is the direction: the regex must never accept a grid
/// `parse_grid` would refuse, because that invents coordinates for a row the calculator has always
/// shown as unrestorable. Accepting *less* is survivable and is what actually happens for four
/// syntactic forms; those four are in the table below, and
/// [`the_backfill_regex_is_narrower_than_parse_grid`] measures the divergence directly.
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
    let cases: [BackfillCase; 10] = [
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
        // ── T-626 — the four forms `parse_grid` accepts and the regex does not.
        //
        // The six cases above are exactly the ones where 0020's "character for character" claim
        // is TRUE, which is why the claim survived: the suite avoided the inputs that break it.
        // These four are those inputs, and they must come back NULL — the regex refuses them, and
        // refusing is the safe direction. `restore()` still reads such a row through `parse_grid`,
        // so nothing is stranded; see `the_backfill_regex_is_narrower_than_parse_grid`.
        ("+1000, 2000", "+2200, 1800", None, None), // `-?` has no `+`
        (".5, 2", ".25, .75", None, None),          // `\d+` wants a digit before the point
        ("5., 2", "6., 3", None, None),             // `(\.\d+)?` wants digits after it
        ("1e3, 500", "2.2e3, 1.8e3", None, None),   // no exponent form
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
    //
    // Comment lines go first: 0020's rationale block contains prose semicolons, and splitting
    // statements before stripping them would shred the file into fragments.
    let sql_only = MIGRATION_0020
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

// ───────────────────────── T-626 — the claim 0020 makes about its own regex ─────────────────────

/// `frontend/src/mortar.rs::parse_grid`, transcribed.
///
/// The frontend is a separate crate (`website-frontend`, built for `wasm32`) and cannot be linked
/// into an API test binary, so the rule is restated here and
/// [`the_transcription_of_parse_grid_is_still_the_shipped_one`] pins every line of it against the
/// shipped source. A transcription nothing checks is how the divergence this test measures got
/// into a comment in the first place.
fn parse_grid(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once(',')?;
    let x: f64 = a.trim().parse().ok()?;
    let y: f64 = b.trim().parse().ok()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

const SHIPPED_MORTAR: &str = include_str!("../../frontend/src/mortar.rs");
const MIGRATION_0020: &str = include_str!("../migrations/0020_fire_missions_solution.sql");

/// The accept regex out of the shipped migration — both copies, which must be the same regex.
///
/// Read from the file rather than retyped for the same reason the backfill statements are: a
/// transcription would agree with itself while the migration said something else.
fn shipped_accept_regex() -> String {
    let patterns: Vec<&str> = MIGRATION_0020
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .filter_map(|l| l.split_once("~ '"))
        .filter_map(|(_, rest)| rest.split_once('\''))
        .map(|(pattern, _)| pattern)
        .collect();
    assert_eq!(
        patterns.len(),
        2,
        "expected two `~ '<regex>'` accept tests in 0020, found {patterns:?}"
    );
    assert_eq!(
        patterns[0], patterns[1],
        "the two backfill statements no longer share one accept regex — `fp` and `tgt` rows would \
         be restored under different rules"
    );
    patterns[0].to_string()
}

/// This suite's own source, so the transcription above can be compared with the shipped function
/// rather than merely asserted to resemble it.
const THIS_SUITE: &str = include_str!("t587_fire_mission_solution.rs");

/// `fn parse_grid`'s source out of `src`, comment lines dropped and whitespace flattened.
///
/// The signature is assembled with `concat!` so this needle does not itself occur as a literal in
/// this file — otherwise it would match its own definition before the function's.
fn parse_grid_source(src: &str, whose: &str) -> String {
    let needle = concat!("fn ", "parse_grid(s: &str) -> Option<(f64, f64)> {");
    let start = src
        .find(needle)
        .unwrap_or_else(|| panic!("{whose} no longer defines `{needle}`"));
    let rest = &src[start..];
    let end = rest
        .find("\n}")
        .unwrap_or_else(|| panic!("{whose}'s parse_grid has no closing brace at column 0"))
        + 2;
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Class-R: the transcribed `parse_grid` above **is** the shipped one, token for token.
///
/// Not "contains these lines" — that would pass while the copy in this file drifted, which is the
/// same shape of defect as the comment T-626 is here to correct: a check that agrees with itself.
#[test]
fn the_transcription_of_parse_grid_is_still_the_shipped_one() {
    assert_eq!(
        parse_grid_source(THIS_SUITE, "this suite"),
        parse_grid_source(SHIPPED_MORTAR, "frontend/src/mortar.rs"),
        "the copy of parse_grid in this file is no longer the shipped one — every assertion about \
         'what the calculator accepts' below is measuring a function nothing ships"
    );
    // …and the corrected claim is where a reader of `parse_grid` will find it, since 0020 is
    // applied + checksummed and its own comment can never be edited.
    assert!(
        SHIPPED_MORTAR.contains("T-626 — what migration `0020`'s backfill regex really accepts"),
        "the T-626 correction is gone from mortar.rs, and 0020's false 'character for character' \
         claim is once again the only description of the accept set"
    );
}

/// **T-626.** 0020 says its regex is `parse_grid`'s "deliberately character for character". It is
/// not. This measures both readers on the same strings and states what is actually true.
///
/// Three claims, each asserted rather than argued:
///
/// 1. **Under-permissive on four syntactic forms** — `+1000, 2000`, `.5, 2`, `5., 2`, `1e3, 500`.
///    `parse::<f64>` takes all four; the regex takes none. This is the safe direction: the row
///    keeps NULL coordinates and `restore()` reads it through `parse_grid` exactly as before.
/// 2. **Never over-permissive in the dangerous direction** — every string the regex accepts,
///    `parse_grid` also accepts, and Postgres's cast lands on the *same* `f64` bits. That is the
///    property that matters: an accept the reader would refuse is an invented coordinate.
/// 3. **One over-permissive class, and it is not a coordinate** — a digit string past `f64::MAX`
///    matches the regex and then fails `::double precision`, which would have aborted the whole
///    migration. `parse_grid` refuses it (`inf` is not finite). No writer can produce one:
///    `fmt_grid`'s longest output is `f64::MAX`'s 309 digits, which casts cleanly.
#[tokio::test]
async fn the_backfill_regex_is_narrower_than_parse_grid() {
    let Some((_app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let regex = shipped_accept_regex();

    // Run the shipped regex in the same engine the migration ran it in.
    async fn regex_accepts(pool: &PgPool, regex: &str, s: &str) -> bool {
        sqlx::query_scalar::<_, bool>("SELECT $1::text ~ $2::text")
            .bind(s)
            .bind(regex)
            .fetch_one(pool)
            .await
            .expect("T-626: evaluate the shipped accept regex")
    }

    // 1 ── the divergence, form by form. Measured, not asserted from the ticket.
    for form in ["+1000, 2000", ".5, 2", "5., 2", "1e3, 500"] {
        assert!(
            parse_grid(form).is_some(),
            "{form:?} must parse in mortar.rs — if it no longer does, the divergence closed and \
             this test is describing history"
        );
        assert!(
            !regex_accepts(&pool, &regex, form).await,
            "0020's regex now accepts {form:?}. That is the DANGEROUS direction: the backfill \
             would write coordinates for rows the calculator shows as unrestorable"
        );
    }

    // …and the form both accept, so the test above is not passing because the regex accepts
    // nothing at all.
    assert!(regex_accepts(&pool, &regex, "1000, 2000").await);
    assert_eq!(parse_grid("1000, 2000"), Some((1000.0, 2000.0)));

    // 2 ── the direction that would be a defect: an accept `parse_grid` refuses. Every accepted
    // string must parse to the same f64 on both sides, bit for bit.
    for (grid, want) in [
        ("1000, 2000", (1000.0_f64, 2000.0_f64)),
        ("2200.5, 1800.25", (2200.5, 1800.25)),
        ("-750, 12800", (-750.0, 12800.0)),
        ("0, 0", (0.0, 0.0)),
        ("  1000 , 2000  ", (1000.0, 2000.0)),
        ("1000,2000", (1000.0, 2000.0)),
        ("0.1, -0.1", (0.1, -0.1)),
    ] {
        assert!(
            regex_accepts(&pool, &regex, grid).await,
            "the regex stopped accepting {grid:?} — rows it has always backfilled would strand"
        );
        assert_eq!(
            parse_grid(grid),
            Some(want),
            "{grid:?} does not parse to {want:?} in mortar.rs"
        );
        // The migration's own arithmetic: `btrim(split_part(...))::double precision`.
        let (x, y): (f64, f64) = sqlx::query_as(
            "SELECT btrim(split_part($1::text, ',', 1))::double precision, \
                    btrim(split_part($1::text, ',', 2))::double precision",
        )
        .bind(grid)
        .fetch_one(&pool)
        .await
        .expect("T-626: cast an accepted grid the way the migration does");
        assert_eq!(
            (x.to_bits(), y.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "{grid:?} backfills to ({x}, {y}) but the calculator reads {want:?} — the same row \
             would say two different things depending on which reader got there first"
        );
    }

    // …and strings both readers refuse stay refused.
    for grid in [
        "012345",
        "AB, CD",
        "1000, ",
        "1000, 2000, 3000",
        "inf, 2",
        "NaN, 2",
    ] {
        assert!(
            !regex_accepts(&pool, &regex, grid).await,
            "regex took {grid:?}"
        );
        assert_eq!(parse_grid(grid), None, "mortar.rs took {grid:?}");
    }

    // 3 ── the pathological edge, recorded for the next reader. 309 nines is past `f64::MAX`.
    let huge = format!("{}, 2", "9".repeat(309));
    assert!(
        regex_accepts(&pool, &regex, &huge).await,
        "a 309-digit grid matches the regex — this is the one input class where it is WIDER than \
         parse_grid, and the consequence is an aborted migration, not a wrong coordinate"
    );
    assert_eq!(
        parse_grid(&huge),
        None,
        "parse_grid must refuse it: `parse::<f64>` overflows to inf and `is_finite` rejects"
    );
    let cast = sqlx::query_scalar::<_, f64>("SELECT btrim(split_part($1::text, ',', 1))::float8")
        .bind(&huge)
        .fetch_one(&pool)
        .await;
    let err = cast
        .expect_err("a 309-nine grid must overflow double precision")
        .to_string();
    assert!(
        err.contains("out of range"),
        "expected an out-of-range cast failure, got: {err}"
    );

    // …and the longest thing `fmt_grid` can actually emit — `f64::MAX` — is fine, which is why
    // no realistic row has the shape above and why 0020 ran without hitting it.
    let f_max = format!("{}, 2", f64::MAX);
    assert_eq!(
        f_max.split_once(',').expect("pair").0.len(),
        309,
        "f64::MAX renders in 309 digits; the boundary above is not hypothetical, it is one digit \
         of headroom"
    );
    assert!(regex_accepts(&pool, &regex, &f_max).await);
    let (x, _): (f64, f64) = sqlx::query_as(
        "SELECT btrim(split_part($1::text, ',', 1))::double precision, \
                btrim(split_part($1::text, ',', 2))::double precision",
    )
    .bind(&f_max)
    .fetch_one(&pool)
    .await
    .expect("f64::MAX must cast cleanly — fmt_grid can emit it");
    assert_eq!(x, f64::MAX);
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
