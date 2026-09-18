//! ORBAT views, slot registration and assignment, squad reservation, member search, and the
//! game-server roster read.
//!
//! The registration path is this file's concurrency gate: a Postgres advisory lock plus a
//! conditional slot claim, so two simultaneous sign-ups for the last seat cannot both win.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::{AuthUser, LeaderUser, ServiceAuth};
use crate::missions::services::mission_compile::flatten_to_mod_document_with_catalog;
use crate::models::{EventMission, EventStatus, OrbatReservation, OrbatSlot, RegistrationState};
use crate::operations::services::event_lookup::{load_em, load_event};
use crate::operations::services::event_status_rules::{
    EFFECTIVE_STATUS_SQL, can_register_status, sql,
};
use crate::operations::services::{OrbatSquadTemplate, parse_orbat_template};
use website_map_engine::data::scenario::wire_safety::{CargoPhys, CargoPhysCatalog};

// --- ORBAT ---

#[derive(Debug, Serialize)]
struct OrbatSlotDto {
    id: String,
    number: i64,
    role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    loadout: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    tag: String,
    slot_index: i64,
    assigned_to: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    assigned_name: String,
}

#[derive(Debug, Serialize)]
struct OrbatSquadDto {
    faction: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    callsign: String,
    squad: String,
    filled: i64,
    total: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by_name: String,
    slots: Vec<OrbatSlotDto>,
}

/// `GET /api/v1/event-missions/:emid/orbat` — ORBAT grouped by squad.
///
/// @route GET /api/v1/event-missions/:emid/orbat
pub async fn get_orbat(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let slots: Vec<OrbatSlot> = sqlx::query_as(
        "SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1 \
         ORDER BY faction ASC, squad ASC, slot_index ASC",
    )
    .bind(em.id)
    .fetch_all(&state.pool)
    .await?;

    let reservations: Vec<OrbatReservation> =
        sqlx::query_as("SELECT * FROM orbat_reservations WHERE event_mission_id = $1")
            .bind(em.id)
            .fetch_all(&state.pool)
            .await?;
    let reserved_by: HashMap<String, String> = reservations
        .into_iter()
        .map(|r| (r.squad, r.reserved_by))
        .collect();

    // Resolve display names for assignees + reservers.
    let mut ids: HashSet<String> = HashSet::new();
    for s in &slots {
        if let Some(a) = &s.assigned_to {
            ids.insert(a.clone());
        }
    }
    for who in reserved_by.values() {
        ids.insert(who.clone());
    }
    let id_vec: Vec<String> = ids.into_iter().collect();
    let names: HashMap<String, String> =
        sqlx::query_as("SELECT discord_id, COALESCE(username, '') AS username FROM users WHERE discord_id = ANY($1)")
            .bind(&id_vec)
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();

    // ══ GROUPED BY (FACTION, SQUAD), NOT SQUAD (T-324) ═════════════════════════════════════
    // Keying on the squad name alone silently merges two factions that field a squad by the same
    // name. Measured on such an ORBAT: `GET .../orbat` returned ONE card labelled
    // `faction: "BLUFOR"`, `total: 4`, holding BLUFOR's pair *and* OPFOR's, with `number` running
    // 1, 2, 1, 2 — and OPFOR absent from the response entirely, so its seats could not be seen or
    // picked at all. `order` deduped the same way (the closure runs once per key), so the second
    // faction had no card to be rendered into.
    //
    // Reachable only once `idx_orbat_slot` covers `faction`: today it is unique on
    // `(event_mission_id, squad, slot_index)`, so attaching two same-named squads fails on
    // duplicate key before this code can be wrong. Fixed here ahead of that index so the widening
    // cannot turn a visible 500 into an invisible wrong-army bug.
    //
    // The reservation key stays the squad NAME — `orbat_reservations` has no faction column, so
    // both factions' cards correctly show the same holder. See [`squad_reserved_by`].
    let mut order: Vec<(String, String)> = Vec::new();
    let mut groups: HashMap<(String, String), OrbatSquadDto> = HashMap::new();
    for s in &slots {
        let key = (s.faction.clone(), s.squad.clone());
        let g = groups.entry(key).or_insert_with(|| {
            order.push((s.faction.clone(), s.squad.clone()));
            let (rb, rbn) = match reserved_by.get(&s.squad) {
                Some(who) => (who.clone(), names.get(who).cloned().unwrap_or_default()),
                None => (String::new(), String::new()),
            };
            OrbatSquadDto {
                faction: s.faction.clone(),
                callsign: s.callsign.clone(),
                squad: s.squad.clone(),
                filled: 0,
                total: 0,
                reserved_by: rb,
                reserved_by_name: rbn,
                slots: Vec::new(),
            }
        });
        let assigned_name = s
            .assigned_to
            .as_ref()
            .and_then(|a| names.get(a).cloned())
            .unwrap_or_default();
        if s.assigned_to.is_some() {
            g.filled += 1;
        }
        g.total += 1;
        g.slots.push(OrbatSlotDto {
            id: s.id.to_string(),
            number: s.slot_index + 1,
            role: s.role.clone(),
            loadout: s.loadout.clone(),
            tag: s.tag.clone(),
            slot_index: s.slot_index,
            assigned_to: s.assigned_to.clone(),
            assigned_name,
        });
    }
    let out: Vec<OrbatSquadDto> = order
        .into_iter()
        .filter_map(|key| groups.remove(&key))
        .collect();
    Ok(Json(json!({ "data": out })))
}

// --- Registration (G7b) ---

/// Release every seat `who` holds in this event-mission except `keep`, and report how many were
/// freed. **The only statement in this file that writes `orbat_slots.assigned_to = NULL.**
///
/// ══ ONE SEAT PER USER PER OPERATION — WHY THIS IS A FUNCTION (T-324) ═══════════════════
/// `event_registrations.slot_id` and `orbat_slots.assigned_to` are a denormalised duplicate of
/// one fact ("which seat is this person in") with no constraint tying them together, so every
/// writer of one has to remember the other. Both claim paths forgot: [`register_for_event_mission`]
/// and [`assign_slot`] each wrote the new seat, repointed the registration at it, and left the
/// previous seat still naming the occupant. Two seats, one registration, and a capacity display
/// reading `assigned_to` that stays wrong until someone withdraws.
///
/// Making the release a named primitive rather than two inline copies is the point. There is now
/// one place the SQL lives, one thing to call before writing a claim, and one docstring to read.
/// It does not make the invariant structural — see the note on [`assign_slot`] — but it removes
/// the failure mode where two handlers drift apart because only one of them was patched.
///
/// `keep: None` releases everything, which is what withdrawal wants. `IS DISTINCT FROM` (not
/// `<>`) is what makes that work: `id <> NULL` is NULL for every row, so a `<>` form would
/// silently release nothing on exactly the path that needs to release all of it.
///
/// Both bounds are load-bearing, and are the ones T-318 established for withdrawal:
///   * `assigned_to = $2` — only seats naming this user. It cannot strip a claim someone else
///     holds, whatever a registration's `slot_id` has drifted onto.
///   * `event_mission_id = $1` — only this operation. Without it, taking a seat in one
///     operation would unseat the user from every other event they are signed up for.
async fn release_other_seats(
    tx: &mut sqlx::PgConnection,
    em_id: Uuid,
    who: &str,
    keep: Option<Uuid>,
) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query(
        "UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL \
         WHERE event_mission_id = $1 AND assigned_to = $2 AND id IS DISTINCT FROM $3",
    )
    .bind(em_id)
    .bind(who)
    .bind(keep)
    .execute(tx)
    .await?
    .rows_affected())
}

/// Who holds the reservation on `squad`, if anyone.
///
/// ══ NAME-SCOPED, NOT FACTION-SCOPED — AND THAT IS A SCHEMA LIMIT (T-324) ═══════════════
/// `orbat_reservations` has **no faction column** (`0001_initial_schema.sql:403-409`; its unique
/// index is `(event_mission_id, squad)`), so a hold on "Alpha 1-1" covers every faction fielding
/// a squad by that name. Today `idx_orbat_slot` is unique on `(event_mission_id, squad,
/// slot_index)` and hides this: two factions cannot both field an "Alpha 1-1" in one operation
/// without a duplicate-key failure on attach. When that index widens to include `faction`, the
/// collision becomes legal and this lookup starts answering for the wrong army — an OPFOR leader
/// cannot reserve their own "Alpha 1-1" (they collide with BLUFOR's), and a BLUFOR hold rejects
/// OPFOR claims with "squad is reserved by a leader".
///
/// That cannot be fixed here. There is no faction recorded on a reservation to compare against,
/// so a `faction` argument would have nothing to filter on; the fix is a migration plus these
/// call sites in one commit, and it is a separate slice. What this function does is make that a
/// **one-place** change: the two gates below share this lookup instead of each carrying their own
/// copy of the SQL. Do not write anything that assumes reservations are faction-aware.
async fn squad_reserved_by<'e, E>(
    ex: E,
    em_id: Uuid,
    squad: &str,
) -> Result<Option<String>, ApiError>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    Ok(sqlx::query_scalar(
        "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em_id)
    .bind(squad)
    .fetch_optional(ex)
    .await?)
}

/// The registration body.
///
/// **`slot_id` is deliberately required — do not add `#[serde(default)]` to it (T-318).**
/// Same shape as T-185 and T-218: the default is not "no data", it decodes as an affirmative
/// empty value and gets bound straight into a write. Here the write is the upsert below, whose
/// `DO UPDATE SET slot_id = EXCLUDED.slot_id` turns an *existing* registration's seat into
/// `NULL` — while `orbat_slots.assigned_to` still names the user. That pair is the orphan: the
/// seat reads as occupied to everyone else, and [`withdraw_from_event_mission`] used to look the
/// seat up *through* the column that was just nulled, so the occupant could not release it
/// either.
///
/// Registering without a seat (bench / waitlist) is still supported — it is `{"slot_id": ""}`,
/// spelled out. `{}` no longer means it. The two are the same to the handler but not to a
/// reader: an empty string is a caller saying "no seat", an absent field is a caller who did not
/// say anything, and only one of those should be allowed to blank a claim. Making it explicit
/// costs a client nothing and turns the most common malformed body into a decode error.
#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    slot_id: String,
}

/// `POST /api/v1/event-missions/:emid/register` — claim a slot / waitlist.
/// Concurrency gate **G7b**: `FOR UPDATE` on the mission row + conditional slot claim.
///
/// ══ THE REGISTRATION WINDOW IS DERIVED, NOT READ ═══════════════════════════════════════
/// The status test here is the one that was broken: it read the stored column, which only
/// `PATCH /events/:id` ever wrote and which nothing moved on a schedule, so `scheduled` and
/// `open` — and therefore sign-ups — persisted indefinitely past the operation itself.
///
/// It now tests [`EFFECTIVE_STATUS_SQL`], evaluated by Postgres against `now()` INSIDE the
/// transaction. Two consequences worth stating:
///   * the window closes on the clock, not on a background task. Registration is refused
///     the first second after `start_time` whether or not the sweep has run, so the fix
///     cannot be undone by the sweeper being slow, wedged, or not deployed.
///   * moving the read into the transaction also closes the old check-then-claim gap where
///     an admin could cancel an event between the guard and the slot write.
///
/// ══ ONE SEAT PER CALLER ════════════════════════════════════════════════════════════════
/// A caller holds at most one `orbat_slots` row per event-mission, and it is the row their
/// `event_registrations.slot_id` names. Claiming a seat releases whatever seat the caller held
/// here first, in the same transaction — see the block above the claim (T-324). No waitlist
/// promotion happens on this path; the reasoning is next to the branch.
///
/// @route POST /api/v1/event-missions/:emid/register
pub async fn register_for_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
    body: Result<Json<RegisterBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let me = &user.discord_id;
    let is_admin = user.role == "admin";
    // `.ok()...unwrap_or_default()` here collapsed *every* extractor failure — malformed JSON, a
    // missing body, the wrong `Content-Type` — into `slot_id: ""`, which is the "no seat" branch,
    // which nulls the caller's existing claim on the way past. A fat-fingered request could
    // therefore orphan a seat with a 200 and no diagnostic anywhere. `map_err` is what the other
    // ~25 handlers in this crate do, including the four other `JsonRejection` sites in this file;
    // this one was the outlier (T-318).
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request("slot_id is required (send \"\" to register without a seat)")
    })?;

    let mut tx = state.pool.begin().await?;
    // Serialize registrations per event mission — the capacity/waitlist decision is
    // check-then-write, so concurrent registrations must queue on the mission row.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;

    // `FOR UPDATE OF e` serializes registrations across the WHOLE event, which the
    // `event_missions` lock above cannot do — see the `max_slots` block below, whose count
    // spans every mission on this event. `OF e` restricts the lock to the `events` row so the
    // `event_missions` subquery inside [`EFFECTIVE_STATUS_SQL`] is not locked too; that is the
    // same form, for the same reason, as the lifecycle convergence sweep.
    //
    // LOCK ORDER is `event_missions` → `events`, and nothing takes the reverse: the only other
    // `events` row lock in the crate is the sweep, which locks `events` alone and merely *reads*
    // `event_missions`. Withdraw and [`assign_slot`] lock `event_missions` alone. Keep it that
    // way — a path that took `events` before `event_missions` would close the cycle.
    let ev_gate: Option<(EventStatus, bool, i64)> = sqlx::query_as(sql(format!(
        "SELECT {} AS status, e.registration_locked, e.max_slots FROM events e \
         WHERE e.id = $1 AND e.deleted_at IS NULL FOR UPDATE OF e",
        &*EFFECTIVE_STATUS_SQL
    )))
    .bind(em.event_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((status, registration_locked, max_slots)) = ev_gate else {
        return Err(ApiError::not_found("event not found"));
    };
    if !can_register_status(status) {
        return Err(ApiError::conflict(
            "registration is closed for this operation",
        ));
    }
    if registration_locked && !is_admin {
        return Err(ApiError::forbidden(
            "registration is locked; an admin must assign you",
        ));
    }

    let capacity: i64 =
        sqlx::query_scalar("SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1")
            .bind(em.id)
            .fetch_one(&mut *tx)
            .await?;
    let registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1 AND state::text = 'registered' AND discord_id <> $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_one(&mut *tx)
    .await?;

    // ══ A SEATLESS OPERATION IS NOT AN UNLIMITED ONE (T-227) ══════════════════════════════
    // The waitlist branch below used to read `capacity > 0 && registered >= capacity`. The
    // `capacity > 0` clause reads like a guard and is the opposite: at `capacity == 0` it
    // switches the capacity check OFF, so every seatless registration was accepted as
    // `registered`, without bound. Refused here, before anything is written.
    //
    // Refused rather than waitlisted, and the distinction is not cosmetic: `Waitlisted` is a
    // promise that a seat may come free, and withdraw promotes against exactly that. No seat can
    // ever come free from an ORBAT that has none, so a waitlist here would be a queue that
    // cannot move. 409 says the true thing.
    //
    // `add_event_mission` refuses to create this state at all, so reaching it means the
    // rows predate that fix or were seeded directly (`seeds/content_golden.sql:638`). Both are
    // real: this is the guard for the data, the attach refusal is the guard for the door.
    if capacity == 0 {
        return Err(ApiError::conflict(
            "this operation has no ORBAT slots, so there is nothing to register for",
        ));
    }

    // ══ `events.max_slots` — THE FIELD THAT LOOKED LOAD-BEARING AND WAS NOT (T-227) ════════
    // It was validated on create (`0..=256`), editable via PATCH, rendered by the SPA as
    // "{max_slots} slot cap" on the Event Hub header (`frontend/src/events.rs:366`) — and read
    // by no decision anywhere. An operator could cap an operation at 8, see the cap on the page,
    // and watch 40 people register.
    //
    // Wired rather than removed, because the UI already promises it and admins already set it;
    // deleting it would quietly withdraw a capability the product appears to have. And note the
    // brief's other option — "the capacity when there is no ORBAT" — is dead code given the
    // refusals above: there is no longer any registerable operation without an ORBAT.
    //
    // What it means: `max_slots` is on `events`, the CONTAINER, while `orbat_slots` belong to one
    // `event_missions` row. They measure different things, so this is a second, event-wide bound
    // and not a duplicate of `capacity` — an operation can field three missions of 40 seats and
    // still cap attendance at 60. `0` remains "no cap" (the column default, and the same
    // threshold the SPA uses to decide whether to render the badge at all).
    //
    // Counted in DISTINCT people, not registrations: signing up for a second mission of the same
    // operation is one person attending one operation, and must not consume a second unit of an
    // attendance cap. Hence `mine` — a caller already registered somewhere on this event adds
    // nobody new and is never refused by it. This count spans missions, which is why the read
    // above took the `events` row `FOR UPDATE`; without that, two registrations on two different
    // missions could both pass the cap in the same instant.
    let (others, mine): (i64, bool) = sqlx::query_as(
        "SELECT count(DISTINCT r.discord_id) FILTER (WHERE r.discord_id <> $2), \
                count(*) FILTER (WHERE r.discord_id = $2) > 0 \
         FROM event_registrations r JOIN event_missions m ON m.id = r.event_mission_id \
         WHERE m.event_id = $1 AND r.state::text = 'registered'",
    )
    .bind(em.event_id)
    .bind(me)
    .fetch_one(&mut *tx)
    .await?;
    if max_slots > 0 && !mine && others >= max_slots {
        return Err(ApiError::conflict(
            "this operation is full — its slot cap has been reached",
        ));
    }

    // The seat this request asks for, resolved BEFORE anything is written so that a
    // syntactically impossible id is still a plain 404 and not a release-then-fail.
    let want: Option<Uuid> = if body.slot_id.is_empty() {
        None
    } else {
        match Uuid::parse_str(&body.slot_id) {
            Ok(sid) => Some(sid),
            Err(_) => return Err(ApiError::not_found("slot not found")),
        }
    };

    // ══ ONE SEAT PER CALLER PER OPERATION — RECONCILED HERE, UNDER THE LOCK (T-324) ═══════
    // Registering used to be claim-only: it wrote the new seat and left the old one claimed.
    // Two *entirely valid* requests — claim slot0, then claim slot1 — therefore left the caller
    // holding two seats while `event_registrations.slot_id` named exactly one. T-318 closed the
    // malformed-body route into that state; this is the larger one, because it needs no mistake
    // at all. Measured before this call existed: both seats `assigned_to` the caller, one
    // registration row, and `GET /events/:id` reporting `filled: 2, registered: 1` on a 2-slot
    // ORBAT — an operation that reads FULL with one person signed up.
    //
    // Placement is the whole fix, not the SQL. It sits INSIDE the transaction that already holds
    // `SELECT ... FOR UPDATE` on the mission row a few statements above, between the capacity
    // read and the conditional claim. Releasing outside that lock — a second request, or even a
    // second statement after `commit` — would swap one wrong answer for a race: another caller
    // could take the seat in the gap and then lose it to our release, or read the ORBAT while the
    // caller momentarily held zero seats. Inside, the release and the claim are one atomic move.
    //
    // On the bench branch `want` is NULL and this releases everything, which is right: that
    // branch nulls the registration's `slot_id` regardless, so leaving `assigned_to` set is
    // precisely the T-318 orphan shape and the caller would have to withdraw entirely to unstick
    // a seat they never meant to keep.
    //
    // Order is release-then-claim, and it must stay that way if a partial unique index on
    // `(event_mission_id, assigned_to)` ever lands: Postgres enforces a unique index per row as
    // it is written, not at end of statement, so claim-then-release would hit the violation on
    // the claim and turn a valid seat move into a 500. Verified against that index, both orders,
    // in a scratch database. Excluding `want` rather than releasing everything also keeps the
    // idempotent own-seat re-claim a single write on one row.
    release_other_seats(&mut tx, em.id, me, want).await?;

    let mut reg_state = RegistrationState::Registered;
    let mut slot_id: Option<Uuid> = None;

    // ══ NO WAITLIST PROMOTION ON THIS PATH, ON PURPOSE ════════════════════════════════════
    // Freeing a seat looks like it should promote whoever is next, and here it must not.
    // Capacity in this handler is counted in *registrations* against the ORBAT slot count
    // (`registered` above), never in occupied seats — so a promotion is owed exactly when the
    // registered head-count drops below capacity. Registering can never do that: the caller
    // keeps their registration row in every branch of this function (the upsert below only ever
    // inserts or updates it), so the head-count is unchanged or one higher, never lower. Moving
    // from slot0 to slot1 is one person still occupying one place in the operation; the released
    // seat was already theirs and was already counted. Promoting for it would seat someone extra
    // against a place that never came free — an over-fill.
    // Even the bench branch cannot owe one: it only reaches `Waitlisted` when
    // `registered >= capacity`, so the caller stepping out of the registered set leaves it still
    // at or above capacity. This is the same reasoning T-318 used to skip promotion when
    // withdraw releases an orphan (no registration row → never counted → nothing to promote),
    // and deliberately the same answer.
    if let Some(sid) = want {
        let slot: Option<OrbatSlot> =
            sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
                .bind(sid)
                .bind(em.id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(slot) = slot else {
            return Err(ApiError::not_found("slot not found"));
        };
        if slot.assigned_to.as_deref().is_some_and(|a| a != me) {
            return Err(ApiError::conflict("slot already taken"));
        }
        // A reserved squad is held for its leader (or an admin). This gate and the one in
        // [`can_manage_squad`] are deliberately NOT the same predicate — here an *unreserved*
        // squad is claimable by anyone, there it is assignable only by an admin — so they share
        // the lookup ([`squad_reserved_by`], which documents the faction limit) and not the
        // decision. Fixing one and missing the other is the trap; there is one query now.
        if !is_admin {
            let res = squad_reserved_by(&mut *tx, em.id, &slot.squad).await?;
            if let Some(rb) = res
                && rb != *me
            {
                return Err(ApiError::conflict("squad is reserved by a leader"));
            }
        }
        // Conditional claim — only a free slot (or the caller's own) is assignable.
        let upd = sqlx::query(
            "UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() \
             WHERE id = $2 AND event_mission_id = $3 AND (assigned_to IS NULL OR assigned_to = $1)",
        )
        .bind(me)
        .bind(sid)
        .bind(em.id)
        .execute(&mut *tx)
        .await?;
        if upd.rows_affected() != 1 {
            return Err(ApiError::conflict("slot already taken"));
        }
        slot_id = Some(sid);
    } else if registered >= capacity {
        // `capacity > 0` used to guard this comparison and was the whole bug (T-227): it did not
        // protect the check, it disabled it. Zero capacity is refused above, so `capacity` is
        // now always ≥ 1 here and the bound is unconditional.
        reg_state = RegistrationState::Waitlisted;
    }

    let (state_out, slot_out): (RegistrationState, Option<Uuid>) = sqlx::query_as(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id, state = EXCLUDED.state \
         RETURNING state, slot_id",
    )
    .bind(em.id)
    .bind(me)
    .bind(slot_id)
    .bind(reg_state)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(
        json!({ "state": state_out.as_str(), "slot_id": slot_out }),
    ))
}

/// `DELETE /api/v1/event-missions/:emid/register` — withdraw + promote waitlist.
///
/// Takes the same `FOR UPDATE` lock on the mission row that [`register_for_event_mission`]
/// does — see the note at the top of the transaction.
///
/// @route DELETE /api/v1/event-missions/:emid/register
pub async fn withdraw_from_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let me = &user.discord_id;

    let mut tx = state.pool.begin().await?;
    // ══ THE SAME LOCK REGISTER TAKES — WITHDRAW WAS THE UNGUARDED HALF (T-324) ════════════
    // Register serialises on this row precisely because the capacity/waitlist decision is
    // check-then-write. Withdraw does the *other* half of that same decision (it deletes a
    // registration and promotes off the waitlist) and took no lock at all, so register's lock
    // only ever excluded other registers. Two concrete losses, both reachable with valid
    // requests:
    //   * two withdrawals at once read the same "oldest waitlisted" row and both promote it —
    //     two seats come free, one person moves up, and the second promotion is simply lost.
    //   * a register that lands on `Waitlisted` because the operation was full, concurrent with
    //     a withdrawal that scans for a waitlisted row before that INSERT commits, finds none:
    //     the seat frees, nobody is promoted, and the new waitlister sits behind a vacancy
    //     until someone else withdraws.
    // Both under-fill rather than over-fill, which is why they are invisible rather than loud.
    // Locking here makes register and withdraw one queue per operation. It is also the same row
    // in the same order in both handlers, so there is no lock-ordering cycle to deadlock on.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;
    // `slot_id` is deliberately NOT selected any more — see the release below.
    let reg: Option<(Uuid, RegistrationState)> = sqlx::query_as(
        "SELECT id, state FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_optional(&mut *tx)
    .await?;

    // ══ RELEASE BY OCCUPANT, NOT BY THE REGISTRATION'S `slot_id` (T-318) ══════════════════
    // This used to be `if let Some(sid) = reg_slot`, i.e. it freed the seat the *registration*
    // pointed at. That reads the seat through a column any registration write can blank, so the
    // one state that most needed releasing — claim held, `slot_id` nulled — was exactly the state
    // it skipped. `assigned_to` is the seat claim itself; it is the only column that has to be
    // true for the seat to read as occupied, so it is the one to key off.
    //
    // It is a broader delete than the old one but a bounded one — see [`release_other_seats`] for
    // why `assigned_to` + `event_mission_id` are the two bounds that keep it from reaching another
    // user's claim or another operation's. On healthy rows it is a subset of the old behaviour and
    // a superset only on the orphans, which is the whole point. `keep: None` because a withdrawal
    // gives up everything: the caller is leaving, not moving.
    let released = release_other_seats(&mut tx, em.id, me, None).await?;

    let Some((reg_id, reg_state)) = reg else {
        // Seats orphaned before this fix ended up here: the no-op withdraw still deleted the
        // registration row, so the occupant's *second* attempt got a 404 and the seat stayed
        // claimed forever. Withdrawing is now allowed to mean "release whatever I hold here",
        // which is what unsticks that backlog without an admin. A caller who genuinely holds
        // nothing released nothing, so they still get the same 404 as before.
        if released == 0 {
            return Err(ApiError::not_found("not registered"));
        }
        tx.commit().await?;
        return Ok(Json(json!({ "withdrawn": true })));
    };
    // No waitlist promotion on that path: an orphan has no registration row, so it was never
    // counted against capacity, and promoting for it would over-fill the operation.
    let was_registered = reg_state == RegistrationState::Registered;
    sqlx::query("DELETE FROM event_registrations WHERE id = $1")
        .bind(reg_id)
        .execute(&mut *tx)
        .await?;
    if was_registered {
        let next: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM event_registrations WHERE event_mission_id = $1 AND state::text = 'waitlisted' \
             ORDER BY registered_at ASC LIMIT 1",
        )
        .bind(em.id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(next_id) = next {
            sqlx::query("UPDATE event_registrations SET state = 'registered' WHERE id = $1")
                .bind(next_id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(Json(json!({ "withdrawn": true })))
}

// --- Slot assignment (leader) ---

/// Admin, or the leader holding this squad. Note this is stricter than the gate in
/// [`register_for_event_mission`]: an *unreserved* squad is freely claimable there and NOT
/// manageable here, so the two share [`squad_reserved_by`] but not the decision. The reservation
/// lookup is name-scoped, not faction-scoped — that limit is the schema's and is documented on
/// [`squad_reserved_by`].
async fn can_manage_squad(
    pool: &PgPool,
    is_admin: bool,
    me: &str,
    em_id: Uuid,
    squad: &str,
) -> bool {
    if is_admin {
        return true;
    }
    let res = squad_reserved_by(pool, em_id, squad).await.ok().flatten();
    res.as_deref() == Some(me)
}

#[derive(Debug, Deserialize)]
pub struct AssignSlotInput {
    #[serde(default)]
    discord_id: String,
}

/// `PUT /api/v1/event-missions/:emid/slots/:slotId/assign` — assign a user (leader/admin).
///
/// @route PUT /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn assign_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
    body: Result<Json<AssignSlotInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let Json(input) = body.map_err(|_| ApiError::bad_request("discord_id required"))?;
    if input.discord_id.is_empty() {
        return Err(ApiError::bad_request("discord_id required"));
    }
    let exists: Option<i32> = sqlx::query_scalar("SELECT 1 FROM users WHERE discord_id = $1")
        .bind(&input.discord_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(ApiError::bad_request("user not found"));
    }
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to assign its slots",
        ));
    }

    let mut tx = state.pool.begin().await?;
    // ══ A LEADER ASSIGNMENT IS A SEAT MOVE TOO (T-324) ════════════════════════════════════
    // The same defect register had, reached by a different door: the claim below writes the new
    // seat, the upsert under it repoints the registration at that seat, and any seat the assignee
    // already held in this operation stayed `assigned_to` them — one person, two seats, one
    // registration row. A leader filling a squad from the member directory is the likeliest way
    // to hit it, because the directory does not show that the person is already seated elsewhere.
    //
    // Confirmed reachable, not inferred: a test drives PUT .../slots/:id/assign against a user
    // already holding another seat in the same operation and asserts they end up with one.
    //
    // The mission-row lock is new here too. Release-then-claim is a check-then-write pair, and
    // register serialises on this row for exactly that reason; without it a leader assignment and
    // a self-registration can interleave between the release and the claim. Same row, same order
    // as the other two handlers, so there is no lock-ordering cycle.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;
    release_other_seats(&mut tx, em.id, &input.discord_id, Some(slot_id)).await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(&input.discord_id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, 'registered') \
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id, state = EXCLUDED.state",
    )
    .bind(em.id)
    .bind(&input.discord_id)
    .bind(slot_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "assigned_to": input.discord_id })))
}

/// `DELETE /api/v1/event-missions/:emid/slots/:slotId/assign` — unassign (leader/admin).
///
/// @route DELETE /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn clear_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to manage its slots",
        ));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1 AND event_mission_id = $2")
        .bind(slot_id)
        .bind(em.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE event_registrations SET slot_id = NULL WHERE event_mission_id = $1 AND slot_id = $2")
        .bind(em.id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "cleared": true })))
}

// --- Squad reservation (leader) ---

#[derive(Debug, Deserialize)]
pub struct SquadBody {
    #[serde(default)]
    squad: String,
}

/// `POST /api/v1/event-missions/:emid/squads/reserve` — hold a squad (leader).
///
/// @route POST /api/v1/event-missions/:emid/squads/reserve
pub async fn reserve_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let me = &leader.0.discord_id;

    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_one(&state.pool)
    .await?;
    if n == 0 {
        return Err(ApiError::not_found("squad not found in this ORBAT"));
    }

    let existing: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(existing) = existing {
        if existing.reserved_by != *me {
            return Err(ApiError::conflict("squad is already reserved"));
        }
        return Ok((StatusCode::OK, Json(existing)).into_response());
    }

    let res: OrbatReservation = sqlx::query_as(
        "INSERT INTO orbat_reservations (event_mission_id, squad, reserved_by) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(em.id)
    .bind(&input.squad)
    .bind(me)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(res)).into_response())
}

/// `POST /api/v1/event-missions/:emid/squads/release` — lift a squad hold (leader/admin).
///
/// @route POST /api/v1/event-missions/:emid/squads/release
pub async fn release_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let res: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    let Some(res) = res else {
        return Err(ApiError::not_found("squad is not reserved"));
    };
    let is_admin = leader.0.role == "admin";
    if res.reserved_by != leader.0.discord_id && !is_admin {
        return Err(ApiError::forbidden(
            "only the reserver or an admin can release this squad",
        ));
    }
    sqlx::query("DELETE FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2")
        .bind(em.id)
        .bind(&input.squad)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "released": true })))
}

// --- Member directory (leader) ---

#[derive(Debug, Serialize)]
struct MemberDto {
    discord_id: String,
    username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MemberQuery {
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Apply the optional `q` substring filter shared by the members COUNT + page SELECTs.
fn push_member_search_filter(qb: &mut QueryBuilder<Postgres>, search: &str) {
    let like = format!("%{search}%");
    qb.push(" AND (username ILIKE ").push_bind(like.clone());
    qb.push(" OR discord_handle ILIKE ")
        .push_bind(like)
        .push(")");
}

/// `GET /api/v1/members` — slim member directory for leaders (excludes banned).
///
/// Offset-paginated via shared [`PageParams`] bounds (default limit 20 / max 100). Response
/// shape matches other list endpoints: `{data, total, limit, offset}`.
///
/// @route GET /api/v1/members
pub async fn search_members(
    State(state): State<AppState>,
    _l: LeaderUser,
    Query(q): Query<MemberQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: q.limit,
        offset: q.offset,
    }
    .bounds();
    let search = q.q.as_deref().filter(|s| !s.is_empty());

    let mut count_qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT count(*) FROM users WHERE is_banned = false");
    if let Some(s) = search {
        push_member_search_filter(&mut count_qb, s);
    }
    let total: i64 = count_qb
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;

    // COALESCE nullable text → '' to mirror Go/GORM scanning NULL into the string zero.
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, ''), COALESCE(avatar_url, '') \
         FROM users WHERE is_banned = false",
    );
    if let Some(s) = search {
        push_member_search_filter(&mut qb, s);
    }
    qb.push(" ORDER BY username ASC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows: Vec<(String, String, String)> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let out: Vec<MemberDto> = rows
        .into_iter()
        .map(|(discord_id, username, avatar_url)| MemberDto {
            discord_id,
            username,
            avatar_url,
        })
        .collect();
    Ok(Json(
        json!({ "data": out, "total": total, "limit": limit, "offset": offset }),
    ))
}

// --- Game-server ingest (service token) ---

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
/// (`SELECT discord_id FROM users WHERE arma_id = $1`, `handlers/telemetry.rs`).
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
/// **T-550 — cargo capacity at roster compile.** Loads the same registry phys table Save /
/// `/compiled` use ([`load_cargo_phys_catalog`]) and compiles via
/// [`flatten_to_mod_document_with_catalog`], so pre-T-416 over-capacity versions are omitted
/// here instead of seating from an empty-catalog no-op. (`load_cargo_phys_catalog` is private
/// in `missions.rs` — duplicated below; owns is events-only.)
///
/// **T-551 — HTTP IT:** `tests/events.rs::roster_omits_over_capacity_mission_when_catalog_loaded`
/// seeds an over-capacity tip (Save-bypass) and asserts the roster stays 200 with that
/// mission's assignments omitted.
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
        // T-529 — filter AND emit via `btrim(u.arma_id)`. `u.arma_id <> ''` alone lets a
        // whitespace-only legacy row through and emits `" "` as a seating key the mod can
        // never resolve (link-confirm + telemetry both trim; T-350 `arma_id_is_linked`
        // treats whitespace as unlinked). Selecting `btrim(...)` keeps the filter and the
        // map key the same expression — no Rust-side one-sided trim (T-343).
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

/// Phys attrs for the T-416 cargo-capacity walk — only columns `scan_cargo_capacity` needs.
///
/// Duplicated from `handlers/missions.rs` (T-550): that helper is private and this slice's
/// owns is `events.rs` only. Keep the SQL and insert shape byte-identical to missions.
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
/// Mirror of `missions::handlers::mission_versions::load_cargo_phys_catalog`. Missing weights /
/// maxima stay `None` (never invent). No current modpack / empty table → empty catalog →
/// cargo walk is a no-op.
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
mod t550_roster_cargo_catalog {
    /// T-550 Class-R: live roster must load the Save phys catalog and compile through the
    /// catalogued gate — empty-catalog `flatten_to_mod_document` lets pre-T-416 over-capacity
    /// versions seat. RED: swap back to the no-arg flatten, or drop the catalog load.
    #[test]
    fn roster_loads_cargo_phys_catalog() {
        const SRC: &str = include_str!("events.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("events.rs must have a #[cfg(test)] module");
        let start = production
            .find("pub async fn ingest_event_roster(")
            .expect("ingest_event_roster must exist");
        let after = &production[start..];
        let body = after
            .split("\n/// Phys attrs for the T-416 cargo-capacity walk")
            .next()
            .expect("ingest_event_roster must precede CargoPhysRow");
        assert!(
            body.contains("load_cargo_phys_catalog"),
            "roster must load registry phys into the catalog; got:\n{body}"
        );
        assert!(
            body.contains("flatten_to_mod_document_with_catalog("),
            "roster must call the catalogued compile gate; got:\n{body}"
        );
        // Isolate so a with_catalog import alone cannot false-green a no-arg call.
        let stripped = body.replace("flatten_to_mod_document_with_catalog", "");
        assert!(
            !stripped.contains("flatten_to_mod_document("),
            "roster must not call the empty-catalog no-arg flatten; got:\n{body}"
        );
    }
}

#[cfg(test)]
mod t412_members_pagination {
    use crate::core::http::pagination::PageParams;

    /// Pure page oracle over a sorted username list — mirrors SQL `ORDER BY username ASC
    /// LIMIT $limit OFFSET $offset`. Member index 20 is invisible at offset 0 and appears
    /// only when offset ≥ 20.
    fn page_usernames<'a>(sorted: &'a [&str], limit: usize, offset: usize) -> Vec<&'a str> {
        sorted.iter().copied().skip(offset).take(limit).collect()
    }

    #[test]
    fn member_at_index_20_requires_offset() {
        let names: Vec<String> = (0..25).map(|i| format!("user_{i:02}")).collect();
        let sorted: Vec<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(sorted.len(), 25);

        let page0 = page_usernames(&sorted, 20, 0);
        assert_eq!(page0.len(), 20);
        assert!(
            !page0.contains(&"user_20"),
            "default first page must not include member at index 20"
        );

        let page_off = page_usernames(&sorted, 20, 20);
        assert!(
            page_off.contains(&"user_20"),
            "offset=20 must surface member at index 20"
        );
        assert_eq!(page_off.first().copied(), Some("user_20"));
    }

    #[test]
    fn page_params_default_limit_is_20_offset_0() {
        assert_eq!(
            PageParams {
                limit: None,
                offset: None
            }
            .bounds(),
            (20, 0)
        );
        assert_eq!(
            PageParams {
                limit: Some(20),
                offset: Some(20)
            }
            .bounds(),
            (20, 20)
        );
    }

    /// Class-R source ratchet: production `search_members` must bind LIMIT + OFFSET (never a
    /// hard-coded `LIMIT 20` with no offset) and emit the list envelope fields.
    #[test]
    fn search_members_binds_limit_offset_and_list_envelope() {
        const SRC: &str = include_str!("events.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("production source before tests module");
        let handler = production
            .split("pub async fn search_members")
            .nth(1)
            .expect("search_members handler")
            .split("\npub async fn ")
            .next()
            .expect("handler body until next pub async fn");
        let collapsed: String = handler.split_whitespace().collect::<Vec<_>>().join(" ");

        assert!(
            !collapsed.contains("ORDER BY username ASC LIMIT 20\""),
            "search_members must not hard-code LIMIT 20 without OFFSET"
        );
        assert!(
            !collapsed.contains("ORDER BY username ASC LIMIT 20"),
            "search_members must not hard-code LIMIT 20 without OFFSET"
        );
        assert!(
            collapsed.contains("push_bind(limit)") && collapsed.contains("push_bind(offset)"),
            "search_members must bind limit and offset parameters"
        );
        assert!(
            collapsed.contains("OFFSET"),
            "search_members SQL must include OFFSET"
        );
        assert!(
            collapsed.contains("\"total\"")
                && collapsed.contains("\"limit\"")
                && collapsed.contains("\"offset\""),
            "search_members must return {{data,total,limit,offset}}"
        );
    }
}

#[cfg(test)]
mod t529_roster_arma_id_btrim {
    /// Class-R: `ingest_event_roster` must filter AND emit via `btrim(u.arma_id)`.
    ///
    /// Perturbation RED: restore `SELECT … u.arma_id` + `u.arma_id <> ''` → asserts fail.
    #[test]
    fn ingest_event_roster_btrims_arma_id_filter_and_select() {
        const SRC: &str = include_str!("events.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("production source before tests module");
        let handler = production
            .split("pub async fn ingest_event_roster")
            .nth(1)
            .expect("ingest_event_roster")
            .split("\npub async fn ")
            .next()
            .expect("handler body until next pub async fn");
        let collapsed: String = handler.split_whitespace().collect::<Vec<_>>().join(" ");

        assert!(
            collapsed.contains("btrim(u.arma_id)"),
            "roster must btrim(u.arma_id) — whitespace-only rows must not seat"
        );
        assert!(
            collapsed.contains("btrim(u.arma_id) <> ''"),
            "roster WHERE must use btrim nonempty, not raw <> ''"
        );
        assert!(
            !collapsed.contains("SELECT os.squad, os.slot_index, u.arma_id"),
            "roster must SELECT btrim(u.arma_id), not raw u.arma_id (emit seating key trimmed)"
        );
        assert!(
            !collapsed.contains("AND u.arma_id <> ''"),
            "roster must not keep the untrimmed <> '' guard (T-529)"
        );
    }
}
