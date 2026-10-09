//! The single-tube columns of `fire_missions` that migration `0020` added, and its coordinate
//! backfill, against the list route `GET /api/v1/events/{id}/fire-missions`.
//!
//! # Why this suite exists in this shape
//!
//! Rows stored before `0020` exist, and the failure mode is not a 500 but a `0` where a `null`
//! belongs. So every assertion below lands on one of two things a handler cannot fake:
//!
//! * the **database row**, read back with a direct `sqlx` query on its own pool connection, not
//!   through any handler;
//! * the **list endpoint's** body, which is built by a `SELECT` — so a column the `SELECT`
//!   forgets to project cannot appear in it.
//!
//! The save path (`POST /api/v1/fire-missions`) re-solves against a stored ballistics catalog;
//! its cases live in `tests/game_ballistics_fire_missions.rs`.
//!
//! # The cases
//!
//! 1. [`a_row_written_before_this_migration_still_lists_and_restores`] — the columns are nullable
//!    because rows predating the migration exist. One is forged directly into the table with all
//!    seven `NULL` and must come back as `null` rather than as `0`, and must not panic the
//!    handler's decode.
//! 2. [`the_shipped_backfill_recovers_coordinates_from_the_grid_encoding`] — runs the migration's
//!    **own** `UPDATE` statements, read out of the shipped `.sql` file, over rows this test
//!    inserts. A transcribed copy of the SQL would test the copy.
//!
//! # The claim case 2 has to check
//!
//! 0020's comment calls its accept regex `parse_grid`'s, "deliberately character for character".
//! It is not: `parse::<f64>` also takes `+1000, 2000`, `.5, 2`, `5., 2` and `1e3, 500`, and the
//! regex takes none of them. A fixture set that omits **exactly** those four forms agrees with the
//! claim by never testing it. Two cases close that:
//!
//! 3. [`the_backfill_regex_is_narrower_than_parse_grid`] — measures both readers on the divergent
//!    forms, on the agreed forms (same `f64` bits, not just "both accept"), and on the one input
//!    class where the regex is the *wider* of the two.
//! 4. [`the_transcription_of_parse_grid_is_still_the_shipped_one`] — case 3 needs a copy of the
//!    calculator's legacy `x, y` reader, `parse_legacy_grid` in
//!    `crates/frontend/pages/field_tools_pages/src/mortar/saved_fires/restore.rs` (the frontend
//!    is a wasm crate and cannot be linked here); this pins the copy against the shipped
//!    function token for token.
//!
//! Every case needs `TEST_DATABASE_URL`, like every DB-backed suite in this crate.

mod common;

use api_configuration::configuration::Config;

use api::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let app = router(api::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "fire-mission-secret"),
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
        .expect("the request builds");
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The stored row, straight out of the table.
///
/// Deliberately **not** the `FireMission` model in `api_operations/src/models/fire_mission.rs` via
/// `query_as`: that struct is what the handler deserialises into, so sharing it would let one
/// wrong column name agree with itself on both sides. Naming the columns here means the test fails if the migration named them differently
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
    .expect("read the stored fire mission back")
}

/// Plants an event for the fire missions to name: saving against, or listing, an event that does
/// not exist answers 404. The creator is the dev-login administrator, so a caller mints its token
/// first.
async fn plant_event(pool: &PgPool) -> String {
    sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, created_by, created_at) \
         VALUES ('Fire mission fixture', now(), '000000000000000001', now()) RETURNING id::text",
    )
    .fetch_one(pool)
    .await
    .expect("plant the event the fire missions name")
}

/// A fire mission saved before this migration must still list and still read as "not recorded".
///
/// The columns are nullable precisely because rows like this exist, and the failure mode is not a
/// 500 — it is a `0` where a `null` belongs. `charge: 0` names a real ring on every tube in
/// `charges_for` and `time_of_flight_s: 0.0` is a plausible flight; either would render on the
/// calculator's card as a confident, wrong, unfalsifiable number for a mission nobody re-checked.
///
/// Forged with a direct INSERT that names only the columns predating the solution migration,
/// which is byte-for-byte the statement such a row was written by.
#[tokio::test]
async fn a_row_written_before_this_migration_still_lists_and_restores() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let tok = common::dev_login_token(&app, "fire_mission_solution", "admin").await;
    let event = plant_event(&pool).await;

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
    .expect("forge a pre-migration fire mission");

    // Nothing filled them in behind our back — no DEFAULT, no trigger.
    let (fp_x, fp_y, tgt_x, tgt_y, az_mils, charge, tof) =
        stored_solution(&pool, &id.to_string()).await;
    assert_eq!(
        (fp_x, fp_y, tgt_x, tgt_y, az_mils, charge, tof),
        (None, None, None, None, None, None, None),
        "a solution column acquired a default — a stored 0 claims to be a measurement"
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
/// claims that and it is false. It is the direction: the regex must never accept a grid
/// `parse_legacy_grid` would refuse, because that invents coordinates for a row the calculator has always
/// shown as unrestorable. Accepting *less* is survivable and is what actually happens for four
/// syntactic forms; those four are in the table below, and
/// [`the_backfill_regex_is_narrower_than_parse_grid`] measures the divergence directly.
#[tokio::test]
async fn the_shipped_backfill_recovers_coordinates_from_the_grid_encoding() {
    let Some((_app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let event = plant_event(&pool).await;

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
        // ── The four forms `parse_legacy_grid` accepts and the regex does not.
        //
        // The six cases above are exactly the ones where 0020's "character for character" claim
        // is TRUE, and a suite that stops there never tests the inputs that break it.
        // These four are those inputs, and they must come back NULL — the regex refuses them, and
        // refusing is the safe direction. `restore()` still reads such a row through
        // `parse_legacy_grid`, so nothing is stranded; see
        // `the_backfill_regex_is_narrower_than_parse_grid`.
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
        .expect("insert a backfill fixture row");
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
        // `event` is the id of an event this test planted, not caller input — that is the whole audit
        // `AssertSqlSafe` is asking for. The scoping exists so a parallel sibling suite's fixture
        // rows in the same database are not rewritten by this replay.
        let scoped = format!("{stmt} AND event_id = '{event}'::uuid");
        sqlx::raw_sql(sqlx::AssertSqlSafe(scoped.clone()))
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("replay the shipped backfill: {e}\n{scoped}"));
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
        .expect("read a backfilled row");
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

// ───────────────────────── the claim 0020 makes about its own regex ─────────────────────────────

/// `crates/frontend/pages/field_tools_pages/src/mortar/saved_fires/restore.rs::parse_legacy_grid`,
/// the calculator's reader of the legacy `x, y` grid text, transcribed.
///
/// The frontend is a separate crate (`frontend`, built for `wasm32`) and cannot be linked
/// into an API test binary, so the rule is restated here and
/// [`the_transcription_of_parse_grid_is_still_the_shipped_one`] pins every line of it against the
/// shipped source. A transcription nothing checks is how the divergence this test measures got
/// into a comment in the first place.
fn parse_legacy_grid(text: &str) -> Option<(f64, f64)> {
    let (a, b) = text.split_once(',')?;
    let x: f64 = a.trim().parse().ok()?;
    let y: f64 = b.trim().parse().ok()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// The shipped source that defines the calculator's legacy grid reader.
const SHIPPED_LEGACY_GRID_READER: &str = include_str!(
    "../../../crates/frontend/pages/field_tools_pages/src/mortar/saved_fires/restore.rs"
);
/// Where [`SHIPPED_LEGACY_GRID_READER`] lives, for failure messages.
const SHIPPED_LEGACY_GRID_READER_PATH: &str =
    "crates/frontend/pages/field_tools_pages/src/mortar/saved_fires/restore.rs";
const MIGRATION_0020: &str =
    include_str!("../../../crates/api/api_database/migrations/0020_fire_missions_solution.sql");

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
const THIS_SUITE: &str = include_str!("fire_mission_solution.rs");

/// `fn parse_legacy_grid`'s source out of `src`, comment lines dropped and whitespace flattened.
///
/// The signature is assembled with `concat!` so this needle does not itself occur as a literal in
/// this file — otherwise it would match its own definition before the function's.
fn parse_legacy_grid_source(src: &str, whose: &str) -> String {
    let needle = concat!(
        "fn ",
        "parse_legacy_grid(text: &str) -> Option<(f64, f64)> {"
    );
    let start = src
        .find(needle)
        .unwrap_or_else(|| panic!("{whose} no longer defines `{needle}`"));
    let rest = &src[start..];
    let end = rest
        .find("\n}")
        .unwrap_or_else(|| panic!("{whose}'s parse_legacy_grid has no closing brace at column 0"))
        + 2;
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The transcribed `parse_legacy_grid` above **is** the shipped one, token for token.
///
/// Not "contains these lines" — that would pass while the copy in this file drifted, which is the
/// same shape of defect as the comment this suite corrects: a check that agrees with itself.
#[test]
fn the_transcription_of_parse_grid_is_still_the_shipped_one() {
    assert_eq!(
        parse_legacy_grid_source(THIS_SUITE, "this suite"),
        parse_legacy_grid_source(SHIPPED_LEGACY_GRID_READER, SHIPPED_LEGACY_GRID_READER_PATH),
        "the copy of parse_legacy_grid in this file is no longer the shipped one — every \
         assertion about 'what the calculator accepts' below is measuring a function nothing ships"
    );
    // …and the corrected claim is where a reader of `parse_legacy_grid` finds it, since 0020 is
    // applied and checksummed and its own comment can never be edited.
    assert!(
        SHIPPED_LEGACY_GRID_READER
            .contains("**The divergence is under-permissive, which is the safe direction.**"),
        "the divergence correction is gone from {SHIPPED_LEGACY_GRID_READER_PATH}, and 0020's \
         false 'character for character' claim is once again the only description of the accept set"
    );
}

/// 0020 says its regex is `parse_grid`'s "deliberately character for character". It is not. This
/// measures both readers on the same strings and states what is actually true.
///
/// Three claims, each asserted rather than argued:
///
/// 1. **Under-permissive on four syntactic forms** — `+1000, 2000`, `.5, 2`, `5., 2`, `1e3, 500`.
///    `parse::<f64>` takes all four; the regex takes none. This is the safe direction: the row
///    keeps NULL coordinates and `restore()` reads it through `parse_legacy_grid` exactly as before.
/// 2. **Never over-permissive in the dangerous direction** — every string the regex accepts,
///    `parse_legacy_grid` also accepts, and Postgres's cast lands on the *same* `f64` bits. That is the
///    property that matters: an accept the reader would refuse is an invented coordinate.
/// 3. **One over-permissive class, and it is not a coordinate** — a digit string past `f64::MAX`
///    matches the regex and then fails `::double precision`, which would have aborted the whole
///    migration. `parse_legacy_grid` refuses it (`inf` is not finite). No writer can produce one:
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
            .expect("evaluate the shipped accept regex")
    }

    // 1 ── the divergence, form by form. Measured, not taken on trust.
    for form in ["+1000, 2000", ".5, 2", "5., 2", "1e3, 500"] {
        assert!(
            parse_legacy_grid(form).is_some(),
            "{form:?} must parse in grid.rs — if it no longer does, the divergence closed and \
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
    assert_eq!(parse_legacy_grid("1000, 2000"), Some((1000.0, 2000.0)));

    // 2 ── the direction that would be a defect: an accept `parse_legacy_grid` refuses. Every accepted
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
            parse_legacy_grid(grid),
            Some(want),
            "{grid:?} does not parse to {want:?} in grid.rs"
        );
        // The migration's own arithmetic: `btrim(split_part(...))::double precision`.
        let (x, y): (f64, f64) = sqlx::query_as(
            "SELECT btrim(split_part($1::text, ',', 1))::double precision, \
                    btrim(split_part($1::text, ',', 2))::double precision",
        )
        .bind(grid)
        .fetch_one(&pool)
        .await
        .expect("cast an accepted grid the way the migration does");
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
        assert_eq!(
            parse_legacy_grid(grid),
            None,
            "parse_legacy_grid took {grid:?}"
        );
    }

    // 3 ── the pathological edge, recorded for the next reader. 309 nines is past `f64::MAX`.
    let huge = format!("{}, 2", "9".repeat(309));
    assert!(
        regex_accepts(&pool, &regex, &huge).await,
        "a 309-digit grid matches the regex — this is the one input class where it is WIDER than \
         parse_legacy_grid, and the consequence is an aborted migration, not a wrong coordinate"
    );
    assert_eq!(
        parse_legacy_grid(&huge),
        None,
        "parse_legacy_grid must refuse it: `parse::<f64>` overflows to inf and `is_finite` rejects"
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
