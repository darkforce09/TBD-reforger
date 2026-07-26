//! Field tools — mortar fire missions + mission injection. Rust port of
//! `handlers/field_tools.go`.

use std::fs;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::missions::build_mission_doc;
use crate::handlers::{load_mission, username};
use crate::middleware::{AdminUser, AuthUser};
use crate::models::{AuditSeverity, FireMission, MissionStatus};
use crate::services::{FireSolution, solve_fire_mission, write_audit};
use crate::state::AppState;

/// Staging dir for injected mission.json files (game-server bridge pickup).
const MISSION_STAGE_DIR: &str = "missions";
/// Local upload storage dir (also served at `/uploads`).
pub(crate) const UPLOAD_DIR: &str = "uploads";

/// The firing-solution request body.
///
/// **All five fields are deliberately required — do not add `#[serde(default)]` to any of them
/// (T-349).** All five carried one until this ticket; this file was skipped by the T-315…T-319
/// sweeps entirely, so it reached T-349 with no annotations and no guards. None of the five is
/// genuinely optional, but they fail in two different ways and the fix differs accordingly.
///
/// **`weapon_system` is a lookup key, and its default was the worst defect in the ticket.** It
/// selects the muzzle-velocity table in [`crate::services::solve_fire_mission`], and
/// `services/mortar.rs:46-52` answers an unknown weapon by *silently substituting*
/// `DEFAULT_MORTAR`. So an absent, misspelled or padded weapon did not fail — it returned a
/// complete, confident firing solution for a **different tube**. Measured on the pre-fix binary
/// at FP (0,0) → TGT (0,3000):
///
/// | request | charge | elevation | TOF |
/// |---|---|---|---|
/// | `"M120 120mm"` | 2 | **1300 mils** | 44.9 s |
/// | `"M120 120mmm"` (one typo) | 3 | **1228 mils** | 40.0 s |
/// | `"M120 120mm "` (one trailing space) | 3 | **1228 mils** | 40.0 s |
/// | `""` / field omitted | 3 | **1228 mils** | 40.0 s |
///
/// A 120mm crew that mistyped its own tube was handed the 81mm elevation — **72 mils** low — on a
/// 200, labelled `M252 81mm`. For a mortar calculator that is not a data-quality nit; it is a
/// round landing somewhere nobody aimed. The default also made it reachable with **no whitespace
/// anywhere in the request**, which is why a trim-only fix would not have closed it.
///
/// **The four coordinates are required for the opposite reason: there is nothing to check.** `0.0`
/// is a perfectly legitimate coordinate — grid origin — so unlike a string there is no "empty"
/// sentinel a guard could look for, and presence is the only question that can be asked. This is
/// exactly the `nav_order = 0` argument T-319 made in this ticket's sibling file
/// ([`super::wiki::WikiInput`]): the fix is *presence*, not non-emptiness, and `0.0` must stay
/// writable. Measured on the pre-fix binary, `{"weapon_system":"M252 81mm","fp_x":1000,"fp_y":2000}`
/// — no target at all — answered **200** with distance 2236 m, azimuth 206.6° and elevation
/// 915 mils: a firing solution onto grid (0,0), a target the caller never named.
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
/// Both `POST /fire-missions/solve` and `POST /fire-missions` used to inline the same
/// solve-then-check-range pair, and only one of them guarded anything. Resolving it once here is
/// the T-347 rule: the guard and the value that gets bound cannot drift apart if there is only
/// one of each.
///
/// **On detecting an unknown weapon without duplicating the weapon table.** `charges_for` is
/// private to `services/mortar.rs` and there is no `is_known_weapon`, so the obvious guard —
/// re-listing the valid weapons here — would plant exactly the drift T-347 closed: a second copy
/// of a table, silently wrong the first time `mortar.rs` gains a tube. Instead this reads
/// `mortar.rs`'s *own answer*: `solve_fire_mission` returns the weapon it actually used, and the
/// only way that differs from the weapon asked for is the `None` arm substituting
/// `DEFAULT_MORTAR`. So `sol.weapon_system != requested` is precisely `charges_for(requested)
/// == None`, computed with zero duplication and no edit needed when the table grows.
///
/// It is indirect, and it should not have to be. **`services/mortar.rs` is not this ticket's file,
/// so the substitution itself is still there**; this guard only stops it escaping through the two
/// HTTP routes. What `mortar.rs` needs, in order of preference: (1) `solve_fire_mission` returns
/// `Result<FireSolution, SolveError>` with distinct `UnknownWeapon` / `OutOfRange` arms, deleting
/// the `None` arm at `mortar.rs:46-52` outright; or (2) failing that, `pub fn
/// is_known_weapon(&str) -> bool` so the check below can be direct. Either way
/// `mortar.rs:21` (`DEFAULT_MORTAR`) loses its only caller and the
/// `unknown_weapon_falls_back` unit test at `mortar.rs:126-131` — which currently *asserts* the
/// substitution as intended behaviour — has to be rewritten to assert the refusal. That test is
/// why this is a `mortar.rs` decision and not one to make from here.
///
/// Order matters: the unknown-weapon 400 comes **before** the out-of-range 422. Pre-fix, a
/// misspelled weapon aimed beyond `DEFAULT_MORTAR`'s reach was answered "target out of range" —
/// a range verdict for a tube the caller never named, about a target that may be well inside the
/// range of the one they did.
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
    let (sol, in_range) = solve_fire_mission(
        &input.weapon_system,
        input.fp_x,
        input.fp_y,
        input.tgt_x,
        input.tgt_y,
    );
    if sol.weapon_system != input.weapon_system {
        return Err(ApiError::bad_request(format!(
            "unknown weapon_system '{}'",
            input.weapon_system
        )));
    }
    if !in_range {
        return Err(ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            "target out of range",
            serde_json::to_value(&sol).unwrap_or(Value::Null),
        ));
    }
    Ok(sol)
}

/// `POST /api/v1/fire-missions/solve` — live firing solution (no persist).
///
/// @route POST /api/v1/fire-missions/solve
pub async fn solve_fire(
    _u: AuthUser,
    body: Result<Json<SolveInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    // Names all five, because after T-349 all five must be *present* — an omitted coordinate now
    // lands here as a decode error, and "invalid body" tells a caller nothing about which of the
    // five it forgot.
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request("weapon_system, fp_x, fp_y, tgt_x and tgt_y are required")
    })?;
    let sol = solve_checked(&input)?;
    Ok(Json(serde_json::to_value(&sol).unwrap()))
}

/// The persist-a-fire-mission body.
///
/// **`fp_grid` and `target_grid` are deliberately required — do not add `#[serde(default)]` to
/// either (T-349).** Both carried one until this ticket. Nothing bad reached the columns through
/// the *absence* path: the guard below rejected `""`, so the default was **masked**. But masked is
/// not absent — this is the T-343 shape (a defaulted `role` sitting one guard away from a
/// privilege write), and the `map_err` below already returns the identical 400 with the identical
/// message for a missing field, so dropping the defaults is invisible on the wire.
///
/// What was *not* masked is the emptiness check itself, which was untrimmed. Measured on the
/// pre-fix binary, `{"fp_grid":"   ","target_grid":"\t"}` answered **201** and stored exactly
/// those bytes: a saved fire mission whose two grid references render as blank cells, that no
/// reader can identify and no author can find again.
///
/// The grids are **trim**-and-stored rather than refused, unlike `weapon_system` above and
/// `wiki_pages.slug` in [`super::wiki`]. The difference is that a grid reference is matched by
/// nothing — no index, no `ON CONFLICT`, no join, no lookup table — it is only ever rendered back
/// out by `GET /events/:id/fire-missions`. With no reader to disagree with, canonicalising a
/// pasted `"012345 "` is a kindness rather than a hazard, which is the same call T-346 made for
/// `item_name` (trim-and-store) against `faction` (refuse, store verbatim) in one struct.
///
/// **`event_id` stays `Option` — it is the one genuinely optional field on this route.** A fire
/// mission need not belong to an event (`fire_missions.event_id` is nullable, and
/// `POST /fire-missions/solve` has no event at all), so `None` is a real state. What it must not
/// mean is "the caller sent something and we could not make sense of it" — see the guard in
/// [`save_fire`] for the 201 that produced.
#[derive(Debug, Deserialize)]
pub struct SaveFireInput {
    #[serde(flatten)]
    solve: SolveInput,
    event_id: Option<String>,
    fp_grid: String,
    target_grid: String,
}

/// `POST /api/v1/fire-missions` — compute + persist a fire mission.
///
/// @route POST /api/v1/fire-missions
pub async fn save_fire(
    State(state): State<AppState>,
    user: AuthUser,
    body: Result<Json<SaveFireInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    // Names all seven, because `solve` is `#[serde(flatten)]`ed into this body: after T-349 a
    // missing `weapon_system` and a missing `fp_grid` are the same decode error arriving here, so
    // a message that only mentions the grids would send a caller hunting for a field they sent.
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
    // `.ok()` here used to swallow the parse failure, and that was the sharpest edge in T-349.
    // Measured on the pre-fix binary: `event_id` with one trailing space, and `event_id`
    // `"not-a-uuid"`, both answered **201 CREATED** and inserted the row with `event_id` NULL.
    // A NULL `event_id` is unreachable from `list_event_fire_missions` below — `WHERE event_id =
    // $1` matches nothing — so the fire mission was permanently invisible to the only endpoint
    // that lists fire missions, on a success response, with the caller's own event id echoed
    // nowhere. The author believes the mission is on the event; the gun line cannot see it.
    //
    // `None` (key absent) and `null` both still mean "no event" and still store NULL. A *present*
    // value now has to parse, and a present-but-blank one is refused rather than quietly demoted
    // to "no event" — if that is what the caller means, they can say it in one less character.
    let event_id = match input.event_id.as_deref() {
        None => None,
        Some(v) if v.trim().is_empty() => {
            return Err(ApiError::bad_request(
                "event_id must not be blank — omit it or send null for no event",
            ));
        }
        Some(v) => Some(Uuid::parse_str(v).map_err(|_| ApiError::bad_request("invalid event_id"))?),
    };
    let fm: FireMission = sqlx::query_as(
        "INSERT INTO fire_missions \
         (event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, azimuth_deg, elevation_mils, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::float8::numeric, $8, now()) \
         RETURNING id, event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, \
          azimuth_deg::float8 AS azimuth_deg, elevation_mils, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
    )
    .bind(event_id)
    .bind(&user.discord_id)
    .bind(&sol.weapon_system)
    .bind(input.fp_grid.trim())
    .bind(input.target_grid.trim())
    .bind(sol.distance_m)
    .bind(sol.azimuth_deg)
    .bind(sol.elevation_mils)
    .fetch_one(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "solution": sol, "fire_mission": fm })),
    ))
}

/// `GET /api/v1/events/:id/fire-missions` — saved fire missions on an event.
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
    let fms: Vec<FireMission> = sqlx::query_as(
        "SELECT id, event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, \
         azimuth_deg::float8 AS azimuth_deg, elevation_mils, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at \
         FROM fire_missions WHERE event_id = $1 ORDER BY created_at ASC",
    )
    .bind(eid)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": fms })))
}

/// `POST /api/v1/missions/:id/inject` — stage mission.json for the server bridge (admin).
///
/// @route POST /api/v1/missions/:id/inject
pub async fn inject_mission(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let Ok(mid) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let m = load_mission(&state.pool, mid)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    if m.status != MissionStatus::Live {
        return Err(ApiError::conflict("only live missions can be injected"));
    }
    let doc = build_mission_doc(&state.pool, &m).await?;
    let data = serde_json::to_vec_pretty(&doc)
        .map_err(|_| ApiError::internal("could not build mission.json"))?;
    fs::create_dir_all(MISSION_STAGE_DIR).map_err(|_| ApiError::internal("staging unavailable"))?;
    let path = format!("{MISSION_STAGE_DIR}/{}.mission.json", m.id);
    fs::write(&path, data).map_err(|_| ApiError::internal("could not stage mission"))?;

    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "mission.inject",
        &format!(
            "{actor_name} injected mission '{}' to the server staging directory",
            m.title
        ),
        "mission",
        &m.id.to_string(),
    )
    .await;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "staged_path": path, "version": doc.version })),
    ))
}
