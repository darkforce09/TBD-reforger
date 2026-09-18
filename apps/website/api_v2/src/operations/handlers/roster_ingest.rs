//! The game-server roster read: the identity → slot map a running server seats players from.
//!
//! Service-token tier. Neither side of the pairing stores the other's id, so the map is
//! recovered by re-running the ORBAT and compile derivations over the mission version payload
//! and lining the two walks up — see [`pair_slots`].

use std::collections::{BTreeMap, HashMap};

use axum::extract::{Path, State};
use axum::response::Json;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
use website_map_engine::data::scenario::wire_safety::{CargoPhys, CargoPhysCatalog};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::ServiceAuth;
use crate::missions::services::mission_compile::flatten_to_mod_document_with_catalog;
use crate::operations::models::event::EventMission;
use crate::operations::services::event_lookup::load_event;
use crate::operations::services::{OrbatSquadTemplate, parse_orbat_template};

/// Pair every materialized `orbat_slots` row of one event mission with the compiled
/// mission slot it stands for, keyed `(squad, slot_index)` → compiled `uid`.
///
/// ══ WHY A PAIRING PASS AND NOT A COLUMN ════════════════════════════════════════════════
/// The two sides of this join were built by different code from the same payload and
/// neither stores the other's id:
///
///   * `orbat_slots` rows are materialized by `materialize_slots` from
///     [`parse_orbat_template`], which carries only `(faction, callsign, squad, role)` +
///     the enumeration index within the squad. **No editor slot id.**
///   * the mod resolves a roster slot id through `TBD_MissionLoader.GetSlotById`
///     (`slot.id == x || slot.uid == x`) against the document `/missions/:id/compiled`
///     served it. `uid` is the editor slot id carried verbatim; `id` is DERIVED
///     (`faction:callsign:role:occurrence`) and shifts under renames/reorders.
///
/// So `uid` is the value to emit, and it has to be recovered by re-running both
/// derivations over the version payload. They are twins: `derive_orbat_from_editor` and
/// `flatten_to_mod_document` both walk `editor.factions` in array order → each
/// `squadIds` → the squad's `slotIds` resolved and sorted by `index`. The flatten skips a
/// squad that resolves to zero slots and the template keeps it, but an empty squad
/// contributes zero rows on BOTH sides, so a flat walk over slots stays in lockstep.
///
/// Two guards make a drift LOUD instead of silent, because a wrong `uid` is not an error
/// the mod can see — `GetSlotById` returns null and `AssignSlotForPlayer` quietly falls
/// through to round-robin, which is the exact bug this route exists to fix:
///   1. the slot totals must agree (they cannot when the stored `orbat_slots` were
///      materialized from a since-superseded version, or from a legacy top-level
///      `orbat[]` array that never matched the editor graph) — mismatch drops the whole
///      mission from the roster rather than emitting plausible-looking wrong ids;
///   2. per slot, the role must agree.
fn pair_slots(
    template: &[OrbatSquadTemplate],
    slots: &[crate::missions::services::mission_compile::ModSlot],
    em_id: Uuid,
) -> HashMap<(String, i64), String> {
    let mut out: HashMap<(String, i64), String> = HashMap::new();
    let template_total: usize = template.iter().map(|s| s.slots.len()).sum();
    if template_total != slots.len() {
        tracing::warn!(
            event_mission = %em_id,
            template_slots = template_total,
            compiled_slots = slots.len(),
            "roster: ORBAT rows and compiled mission disagree on slot count — omitting this \
             mission from the roster (re-attach it to re-materialize its ORBAT)",
        );
        return out;
    }

    let mut cursor = 0usize;
    for sq in template {
        for (i, sl) in sq.slots.iter().enumerate() {
            let compiled = &slots[cursor];
            cursor += 1;
            // The flatten substitutes this for a slot with no authored role, so compare
            // against the substituted value or every roleless slot reads as a mismatch.
            let want = if sl.role.is_empty() {
                "unassigned"
            } else {
                sl.role.as_str()
            };
            if compiled.role != want {
                tracing::warn!(
                    event_mission = %em_id,
                    squad = %sq.squad,
                    slot_index = i,
                    orbat_role = %want,
                    compiled_role = %compiled.role,
                    "roster: ORBAT row does not line up with the compiled slot — skipped",
                );
                continue;
            }
            out.insert((sq.squad.clone(), i as i64), compiled.uid.clone());
        }
    }
    out
}

/// `GET /api/v1/ingest/events/:id/roster` — identity → slot map for a running event
/// (service-token tier).
///
/// ══ THE KEY IS `users.arma_id`, AND THAT IS LOAD-BEARING ═══════════════════════════════
/// The mod looks a player up with `TBD_RosterLoader.GetSlotForIdentity(bindKey)`, where
/// `bindKey` is `TBD_SpawnManager.PlayerBindKey` =
/// `string.Format("%1", SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId))` — the raw
/// engine identity UUID — and `ResolveSlotIdForPlayer` refuses anything not durable
/// (`player:<id>` leases and vanilla's synthesized `00bbbddd-` name hashes never reach the
/// lookup). That is byte-identical to `TBD_PlayerIdentity.GetArmaId`, which is the ONLY
/// thing the mod ever puts on the wire as an identity, and the only thing besides the dev
/// seed that ever writes `users.arma_id` is `POST /api/v1/ingest/link-confirm`
/// ([`crate::identity_and_access::handlers::arma_link_confirmation::ingest_link_confirm`]) writing exactly that value. The results
/// ingest resolves the same column the same way
/// (`SELECT discord_id FROM users WHERE arma_id = $1`, `match_telemetry/handlers/match_results.rs`).
///
/// Any other column here — `discord_id`, `arma_character`, the `orbat_slots` UUID — would
/// match nobody, forever, and the failure is INVISIBLE: an unmatched key simply never gets
/// looked up and every player falls through to round-robin seating with a 200 on the wire.
///
/// The roster covers every mission attached to the event, because the mod does not tell us
/// which one the server is running. Assignments for a mission the server did not load are
/// harmless (their `uid` resolves to nothing there); a player registered on two missions of
/// one event is resolved deterministically — earliest mission by start time wins.
///
/// **Cargo capacity at roster compile.** Loads the same registry phys table Save and
/// `/compiled` use ([`load_cargo_phys_catalog`]) and compiles through
/// [`flatten_to_mod_document_with_catalog`], so an over-capacity version is omitted here
/// instead of being seated from an empty-catalog no-op.
///
/// `tests/events.rs::roster_omits_over_capacity_mission_when_catalog_loaded` seeds an
/// over-capacity tip (Save-bypass) and asserts the roster stays 200 with that mission's
/// assignments omitted.
///
/// @route GET /api/v1/ingest/events/:id/roster
pub async fn ingest_event_roster(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let ev = load_event(&state.pool, &id).await?;

    let ems: Vec<EventMission> = sqlx::query_as(
        "SELECT id, event_id, mission_id, start_time, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
         FROM event_missions WHERE event_id = $1 ORDER BY start_time ASC, id ASC",
    )
    .bind(ev.id)
    .fetch_all(&state.pool)
    .await?;

    // Same phys table as Save / GET /compiled — empty catalog stays silent (never invent).
    // Loaded once for the whole roster; current-modpack table does not vary per mission.
    let catalog = load_cargo_phys_catalog(&state.pool).await?;

    // Serialized as a JSON object of identity → slot uid. BTreeMap so the body is stable
    // across calls (the mod diffs nothing, but a stable body makes a capture diffable).
    let mut assignments: BTreeMap<String, String> = BTreeMap::new();

    for em in &ems {
        let Some(mission) =
            crate::missions::services::mission_lookup::load_mission(&state.pool, em.mission_id)
                .await?
        else {
            continue;
        };
        let Some(vid) = mission.current_version_id else {
            continue;
        };
        let payload: Option<crate::core::wire_format::RawJson> =
            sqlx::query_scalar("SELECT json_payload FROM mission_versions WHERE id = $1")
                .bind(vid)
                .fetch_optional(&state.pool)
                .await?;
        let Some(payload) = payload else {
            continue;
        };
        let bytes = payload.0.get().as_bytes();

        let doc = match flatten_to_mod_document_with_catalog(&mission, bytes, &catalog) {
            Ok(doc) => doc,
            // A mission with no placed slots (or cargo refuse) has no seats to hand out; the
            // mod's own `/compiled` fetch answers 409 / 500 for those cases too.
            Err(e) => {
                tracing::warn!(
                    event_mission = %em.id,
                    mission = %em.mission_id,
                    error = ?e,
                    "roster: mission does not compile — omitted",
                );
                continue;
            }
        };
        let by_key = pair_slots(&parse_orbat_template(bytes), &doc.slots, em.id);
        if by_key.is_empty() {
            continue;
        }

        // `assigned_to` is the seat claim itself: every writer sets it together with the
        // matching `event_registrations` row (self-register claims it conditionally,
        // `assign_slot` writes both, withdraw/`clear_slot` null both), and it is the
        // column the conditional-claim guard reads. Driving off it therefore covers
        // leader-assigned and self-registered seats alike, and cannot serve a waitlisted
        // player a seat they never got.
        //
        // Filter AND emit via `btrim(u.arma_id)`. `u.arma_id <> ''` alone lets a
        // whitespace-only row through and emits `" "` as a seating key the mod can never
        // resolve (link-confirm and telemetry both trim; `arma_id_is_linked` treats
        // whitespace as unlinked). Selecting `btrim(...)` keeps the filter and the map key
        // the same expression, with no Rust-side one-sided trim.
        let claims: Vec<(String, i64, String)> = sqlx::query_as(
            "SELECT os.squad, os.slot_index, btrim(u.arma_id) \
             FROM orbat_slots os \
             JOIN users u ON u.discord_id = os.assigned_to \
             WHERE os.event_mission_id = $1 AND os.assigned_to IS NOT NULL \
               AND u.arma_id IS NOT NULL AND btrim(u.arma_id) <> '' \
               AND u.deleted_at IS NULL",
        )
        .bind(em.id)
        .fetch_all(&state.pool)
        .await?;

        for (squad, slot_index, arma_id) in claims {
            let Some(uid) = by_key.get(&(squad, slot_index)) else {
                continue;
            };
            // First mission by start time wins — see the doc comment.
            assignments.entry(arma_id).or_insert_with(|| uid.clone());
        }
    }

    // `TBD_RosterResponseStruct` declares `eventId`, `missionId` and `assignments`, so the
    // keys are camelCase here and NOT the snake_case API contract — Enfusion's
    // `JsonLoadContext` binds JSON keys to class fields by name and silently ignores any
    // key the class does not declare. `missionId` is informational (the mod reads only
    // `eventId`, to warn on a proxy/config mix-up) and is only meaningful when the event
    // holds exactly one mission.
    let mission_id = match ems.as_slice() {
        [only] => only.mission_id.to_string(),
        _ => String::new(),
    };
    Ok(Json(json!({
        "eventId": ev.id.to_string(),
        "missionId": mission_id,
        "assignments": assignments,
    })))
}

/// Phys attrs for the cargo-capacity walk — only the columns `scan_cargo_capacity` needs.
///
/// The mission domain holds a row of the same shape for its own compile path. Keep the SQL and
/// the insert shape identical to it.
#[derive(sqlx::FromRow)]
struct CargoPhysRow {
    resource_name: String,
    display_name: String,
    weight_kg: Option<f64>,
    volume_cm3: Option<f64>,
    max_weight_kg: Option<f64>,
    max_volume_cm3: Option<f64>,
}

/// Load `CargoPhysCatalog` from the **current** modpack's `registry_items`.
///
/// Mirror of `missions::handlers::mission_versions::load_cargo_phys_catalog`, kept in this
/// domain so the roster read depends on the mission domain's services and not on its handlers.
/// Missing weights / maxima stay `None` (never invent). No current modpack / empty table →
/// empty catalog → cargo walk is a no-op.
async fn load_cargo_phys_catalog(pool: &PgPool) -> Result<CargoPhysCatalog, ApiError> {
    let rows: Vec<CargoPhysRow> = sqlx::query_as(
        "SELECT ri.resource_name, ri.display_name, \
                ri.weight_kg, ri.volume_cm3, ri.max_weight_kg, ri.max_volume_cm3 \
         FROM registry_items ri \
         INNER JOIN modpacks m ON m.id = ri.modpack_id \
         WHERE m.is_current = true",
    )
    .fetch_all(pool)
    .await?;
    let mut catalog = CargoPhysCatalog::with_capacity(rows.len());
    for r in rows {
        catalog.insert(
            r.resource_name,
            CargoPhys {
                display_name: r.display_name,
                weight_kg: r.weight_kg,
                volume_cm3: r.volume_cm3,
                max_weight_kg: r.max_weight_kg,
                max_volume_cm3: r.max_volume_cm3,
            },
        );
    }
    Ok(catalog)
}

#[cfg(test)]
#[path = "tests/roster_ingest.rs"]
mod tests;
