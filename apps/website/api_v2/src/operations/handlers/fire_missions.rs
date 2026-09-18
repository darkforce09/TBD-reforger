//! Field tools — the mortar fire-mission calculator: a live firing solution, the persisted
//! fire-mission row, and the per-event list the gun line reads back.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::operations::models::fire_mission::FireMission;
use website_map_engine::data::scenario::ballistics::{
    FireSolution, SolveError, solve_fire_mission,
};

/// The firing-solution request body.
///
/// **All five fields are deliberately required — do not add `#[serde(default)]` to any of them.**
/// None of the five is genuinely optional, but they fail in two different ways and the guards
/// differ accordingly.
///
/// **`weapon_system` is a lookup key, and a default on it is the sharpest defect this struct can
/// carry.** It selects the muzzle-velocity table in
/// [`website_map_engine::data::scenario::ballistics::solve_fire_mission`]. With a default, an
/// absent, misspelled or padded weapon does not fail — it yields a complete, confident firing
/// solution for a **different tube**. At FP (0,0) → TGT (0,3000), `"M120 120mm"` solves to charge
/// 2 / 1300 mils / 44.9 s TOF, while `"M120 120mmm"`, `"M120 120mm "` and `""` all fall through to
/// charge 3 / 1228 mils / 40.0 s: a 120mm crew that mistypes its own tube is handed the 81mm
/// elevation, **72 mils** low, labelled `M252 81mm`. For a mortar calculator that is not a
/// data-quality nit; it is a round landing somewhere nobody aimed. It is also reachable with **no
/// whitespace anywhere in the request**, which is why trimming alone does not close it.
///
/// **The four coordinates are required for the opposite reason: there is nothing to check.** `0.0`
/// is a perfectly legitimate coordinate — grid origin — so unlike a string there is no "empty"
/// sentinel a guard could look for, and presence is the only question that can be asked. This is
/// the same `nav_order = 0` argument that governs
/// [`crate::community_content::handlers::wiki_knowledgebase::WikiInput`]: the fix is *presence*,
/// not non-emptiness, and `0.0` must stay writable. Without it,
/// `{"weapon_system":"M252 81mm","fp_x":1000,"fp_y":2000}` — no target at all — answers **200**
/// with distance 2236 m, azimuth 206.6° and elevation 915 mils: a firing solution onto grid (0,0),
/// a target the caller never named.
#[derive(Debug, Deserialize)]
pub struct SolveInput {
    weapon_system: String,
    fp_x: f64,
    fp_y: f64,
    tgt_x: f64,
    tgt_y: f64,
}

/// Guard the weapon, then solve — the one place either handler is allowed to reach the ballistics.
///
/// `POST /fire-missions/solve` and `POST /fire-missions` share it rather than each inlining a
/// solve-then-check-range pair: the guard and the value that gets bound cannot drift apart if
/// there is only one of each.
///
/// Order matters: the unknown-weapon 400 comes **before** the out-of-range 422. A misspelled
/// weapon aimed beyond a substituted tube's reach must not be answered "target out of range" — a
/// range verdict for a tube the caller never named, about a target that may be well inside the
/// range of the one they did. That ordering is *structural* rather than conventional: an unknown
/// weapon has no charge table and so never reaches the range loop, leaving no range verdict in
/// existence to report first.
fn solve_checked(input: &SolveInput) -> Result<FireSolution, ApiError> {
    if input.weapon_system.trim().is_empty() {
        return Err(ApiError::bad_request("weapon_system is required"));
    }
    // Refused, not silently canonicalised. `"M120 120mm "` is a weapon this API does not have,
    // and guessing which one the caller meant is how you end up computing 81mm numbers for a
    // 120mm tube — the exact bug this guard exists to stop.
    if input.weapon_system != input.weapon_system.trim() {
        return Err(ApiError::bad_request(
            "weapon_system must not have leading or trailing whitespace",
        ));
    }
    // `UnknownWeapon` carries the weapon verbatim, so the 400 body names exactly what the caller
    // sent. `OutOfRange` carries the PARTIAL solution — distance and azimuth are computed and
    // correct — and it is serialised into the 422's `details`, which is why the variant carries a
    // payload at all (`website_map_engine::data::scenario::ballistics`). Do not collapse it to a
    // message.
    match solve_fire_mission(
        &input.weapon_system,
        input.fp_x,
        input.fp_y,
        input.tgt_x,
        input.tgt_y,
    ) {
        Ok(sol) => Ok(sol),
        Err(SolveError::UnknownWeapon(w)) => Err(ApiError::bad_request(format!(
            "unknown weapon_system '{w}'"
        ))),
        Err(SolveError::OutOfRange(sol)) => Err(ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            "target out of range",
            serde_json::to_value(&sol).unwrap_or(Value::Null),
        )),
    }
}

/// `POST /api/v1/fire-missions/solve` — live firing solution (no persist).
///
/// @route POST /api/v1/fire-missions/solve
pub async fn solve_fire(
    _u: AuthUser,
    body: Result<Json<SolveInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    // Names all five, because all five must be *present* — an omitted coordinate lands here as a
    // decode error, and "invalid body" tells a caller nothing about which of the five it forgot.
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request("weapon_system, fp_x, fp_y, tgt_x and tgt_y are required")
    })?;
    let sol = solve_checked(&input)?;
    Ok(Json(serde_json::to_value(&sol).unwrap()))
}

/// The persist-a-fire-mission body.
///
/// **`fp_grid` and `target_grid` are deliberately required — do not add `#[serde(default)]` to
/// either.** A default on them is masked by the emptiness guard below rather than absent, which is
/// the same shape as a defaulted `role` sitting one guard away from a privilege write; the
/// `map_err` below already returns the identical 400 with the identical message for a missing
/// field, so requiring them is invisible on the wire.
///
/// The emptiness check is a **trimmed** one, because an untrimmed one lets
/// `{"fp_grid":"   ","target_grid":"\t"}` answer **201** and store exactly those bytes: a saved
/// fire mission whose two grid references render as blank cells, that no reader can identify and
/// no author can find again.
///
/// The grids are **trim**-and-stored rather than refused, unlike `weapon_system` above and
/// `wiki_pages.slug` in [`crate::community_content::handlers::wiki_knowledgebase`]. The difference
/// is that a grid reference is matched by nothing — no index, no `ON CONFLICT`, no join, no lookup
/// table — it is only ever rendered back out by `GET /events/:id/fire-missions`. With no reader to
/// disagree with, canonicalising a pasted `"012345 "` is a kindness rather than a hazard, which is
/// the same call `item_name` (trim-and-store) against `faction` (refuse, store verbatim) makes in
/// one struct.
///
/// **`event_id` stays `Option` — it is the one genuinely optional field on this route.** A fire
/// mission need not belong to an event (`fire_missions.event_id` is nullable, and
/// `POST /fire-missions/solve` has no event at all), so `None` is a real state. What it must not
/// mean is "the caller sent something and we could not make sense of it" — see the guard in
/// [`save_fire`].
#[derive(Debug, Deserialize)]
pub struct SaveFireInput {
    #[serde(flatten)]
    solve: SolveInput,
    event_id: Option<String>,
    fp_grid: String,
    target_grid: String,
}

/// Every column of `fire_missions`, spelled once.
///
/// The INSERT's `RETURNING` and the list `SELECT` must project the identical set — they both
/// deserialise into [`FireMission`], so a column added to one and missed on the other is a decode
/// error on exactly one of the two routes. With sixteen columns to keep in step, a copy is a
/// matter of time.
///
/// `azimuth_deg` needs its `::float8` because the column is `numeric(5,1)`. The seven solution
/// columns are already `double precision` / `bigint` (see `0020_fire_missions_solution.sql` on
/// why) and cast to nothing. `created_at` is `COALESCE`d because it is nullable in the shipped
/// schema while the model types it non-`Option`.
///
/// **A macro rather than a `const &str`, because sqlx 0.9 takes `SqlSafeStr`.** That trait is
/// implemented for `&'static str` only; a `format!`ed query needs `AssertSqlSafe`, which is the
/// injection-audit escape hatch and has no business wrapping a query with no runtime input in it.
/// Expanding to a literal inside `concat!` keeps both call sites on the `&'static str` path with
/// the safety check intact and the list written once.
macro_rules! fire_mission_columns {
    () => {
        "id, event_id, created_by, weapon_system, fp_grid, target_grid, \
         distance_m, azimuth_deg::float8 AS azimuth_deg, elevation_mils, fp_x, fp_y, tgt_x, tgt_y, \
         azimuth_mils, charge, time_of_flight_s, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at"
    };
}

/// `POST /api/v1/fire-missions` — compute + persist a fire mission.
///
/// All fifteen values go in. `sol` is the single source for the seven computed ones — they are
/// bound straight off the struct `solve_checked` returned, so the row and the response body cannot
/// disagree about a number — and `input.solve` is the source for the four coordinates, stored as
/// the caller sent them. A row that carried only `distance_m`, `azimuth_deg` and `elevation_mils`
/// would be honest about what it *computed* and silent about how little survived the statement:
/// read it back an hour later and the charge ring, the sight setting and the time to splash are
/// gone.
///
/// @route POST /api/v1/fire-missions
pub async fn save_fire(
    State(state): State<AppState>,
    user: AuthUser,
    body: Result<Json<SaveFireInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    // Names all seven, because `solve` is `#[serde(flatten)]`ed into this body: a missing
    // `weapon_system` and a missing `fp_grid` are the same decode error arriving here, so a
    // message that only mentions the grids would send a caller hunting for a field they sent.
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request(
            "weapon_system, fp_x, fp_y, tgt_x, tgt_y, fp_grid and target_grid are required",
        )
    })?;
    if input.fp_grid.trim().is_empty() || input.target_grid.trim().is_empty() {
        return Err(ApiError::bad_request(
            "fp_grid and target_grid are required",
        ));
    }
    let sol = solve_checked(&input.solve)?;
    // The parse failure is surfaced, never swallowed with `.ok()`. An `event_id` with one trailing
    // space, or `"not-a-uuid"`, would otherwise answer **201 CREATED** and insert the row with
    // `event_id` NULL. A NULL `event_id` is unreachable from `list_event_fire_missions` below —
    // `WHERE event_id = $1` matches nothing — so the fire mission would be permanently invisible
    // to the only endpoint that lists fire missions, on a success response, with the caller's own
    // event id echoed nowhere. The author believes the mission is on the event; the gun line
    // cannot see it.
    //
    // `None` (key absent) and `null` both mean "no event" and store NULL. A *present* value has to
    // parse, and a present-but-blank one is refused rather than quietly demoted to "no event" — if
    // that is what the caller means, they can say it in one less character.
    let event_id = match input.event_id.as_deref() {
        None => None,
        Some(v) if v.trim().is_empty() => {
            return Err(ApiError::bad_request(
                "event_id must not be blank — omit it or send null for no event",
            ));
        }
        Some(v) => Some(Uuid::parse_str(v).map_err(|_| ApiError::bad_request("invalid event_id"))?),
    };
    let fm: FireMission = sqlx::query_as(concat!(
        "INSERT INTO fire_missions \
         (event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, azimuth_deg, \
          elevation_mils, fp_x, fp_y, tgt_x, tgt_y, azimuth_mils, charge, time_of_flight_s, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::float8::numeric, $8, $9, $10, $11, $12, $13, $14, $15, now()) \
         RETURNING ",
        fire_mission_columns!()
    ))
    .bind(event_id)
    .bind(&user.discord_id)
    .bind(&sol.weapon_system)
    .bind(input.fp_grid.trim())
    .bind(input.target_grid.trim())
    .bind(sol.distance_m)
    .bind(sol.azimuth_deg)
    .bind(sol.elevation_mils)
    // The four coordinates as the caller sent them — `solve_checked` does not modify them, and
    // storing the request rather than a round-trip through the solution is what makes the row
    // re-solvable byte for byte.
    .bind(input.solve.fp_x)
    .bind(input.solve.fp_y)
    .bind(input.solve.tgt_x)
    .bind(input.solve.tgt_y)
    // The three computed values bound off `sol`, the same struct serialised into the response
    // below, so the row cannot disagree with what the caller was told.
    .bind(sol.azimuth_mils)
    .bind(sol.charge)
    .bind(sol.time_of_flight_s)
    .fetch_one(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "solution": sol, "fire_mission": fm })),
    ))
}

/// `GET /api/v1/events/:id/fire-missions` — saved fire missions on an event.
///
/// **This is the only reader of `fire_missions`.** Rows written before migration `0020` come back
/// with `null` in all seven solution fields — the calculator renders `—` for those — and rows
/// written after come back with the charge, the sight setting and the time of flight the crew
/// needs. `ORDER BY created_at ASC`: the SPA takes the last row as the newest.
///
/// @route GET /api/v1/events/:id/fire-missions
pub async fn list_event_fire_missions(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Ok(eid) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let fms: Vec<FireMission> = sqlx::query_as(concat!(
        "SELECT ",
        fire_mission_columns!(),
        " FROM fire_missions WHERE event_id = $1 ORDER BY created_at ASC"
    ))
    .bind(eid)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": fms })))
}
