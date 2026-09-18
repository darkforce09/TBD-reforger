//! Self-service registration on one event mission: claiming a seat, taking the bench, and
//! withdrawing.
//!
//! This is the domain's concurrency gate. Both handlers run inside a transaction that holds a
//! `FOR UPDATE` lock on the `event_missions` row, so the capacity / waitlist decision and the
//! seat claim that follows it cannot interleave with another sign-up or a withdrawal, and two
//! simultaneous claims on the last seat cannot both win.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Postgres;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::operations::models::event::{EventStatus, OrbatSlot, RegistrationState};
use crate::operations::services::event_lookup::load_em;
use crate::operations::services::event_status_rules::{
    EFFECTIVE_STATUS_SQL, can_register_status, sql,
};

/// Release every seat `who` holds in this event-mission except `keep`, and report how many were
/// freed. **Every path that gives up or moves a seat writes
/// `orbat_slots.assigned_to = NULL` through this one statement**, apart from `clear_slot`, which
/// nulls one named seat directly.
///
/// ══ ONE SEAT PER USER PER OPERATION — WHY THIS IS A FUNCTION ═══════════════════════════
/// `event_registrations.slot_id` and `orbat_slots.assigned_to` are a denormalised duplicate of
/// one fact ("which seat is this person in") with no constraint tying them together, so every
/// writer of one has to remember the other. A claim path that writes the new seat and repoints
/// the registration at it, without releasing the seat its occupant already held, leaves two
/// seats against one registration and a capacity display reading `assigned_to` that stays wrong
/// until someone withdraws. Both `register_for_event_mission` and `assign_slot` are such paths.
///
/// Keeping the release a named primitive rather than one inline copy per claim path is the
/// point: one place the SQL lives, one thing to call before writing a claim, and one docstring
/// to read. It does not make the invariant structural — see the note on `assign_slot` — but it
/// removes the failure mode where two handlers drift apart because only one of them is changed.
///
/// `keep: None` releases everything, which is what withdrawal wants. `IS DISTINCT FROM` (not
/// `<>`) is what makes that work: `id <> NULL` is NULL for every row, so a `<>` form would
/// silently release nothing on exactly the path that needs to release all of it.
///
/// Both bounds are load-bearing:
///   * `assigned_to = $2` — only seats naming this user. It cannot strip a claim someone else
///     holds, whatever a registration's `slot_id` has drifted onto.
///   * `event_mission_id = $1` — only this operation. Without it, taking a seat in one
///     operation would unseat the user from every other event they are signed up for.
pub(super) async fn release_other_seats(
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
/// ══ NAME-SCOPED, NOT FACTION-SCOPED — AND THAT IS A SCHEMA LIMIT ══════════════════════
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
/// so a `faction` argument would have nothing to filter on; the fix is a migration plus the call
/// sites in one change. What this function does is make that a **one-place** change: the
/// self-service claim gate and the leader-management gate share this lookup instead of each
/// carrying their own copy of the SQL. Do not write anything that assumes reservations are
/// faction-aware.
pub(super) async fn squad_reserved_by<'e, E>(
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
/// **`slot_id` is deliberately required — do not add `#[serde(default)]` to it.**
/// The default is not "no data": it decodes as an affirmative empty value and gets bound
/// straight into a write. Here the write is the upsert below, whose
/// `DO UPDATE SET slot_id = EXCLUDED.slot_id` turns an *existing* registration's seat into
/// `NULL` — while `orbat_slots.assigned_to` still names the user. That pair is the orphan: the
/// seat reads as occupied to everyone else, and a withdrawal that looked the seat up *through*
/// the column just nulled could not release it either.
///
/// Registering without a seat (bench / waitlist) is still supported — it is `{"slot_id": ""}`,
/// spelled out. `{}` does not mean it. The two are the same to the handler but not to a
/// reader: an empty string is a caller saying "no seat", an absent field is a caller who did not
/// say anything, and only one of those should be allowed to blank a claim. Making it explicit
/// costs a client nothing and turns the most common malformed body into a decode error.
#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    slot_id: String,
}

/// `POST /api/v1/event-missions/:emid/register` — claim a slot / waitlist.
///
/// The concurrency gate is a `FOR UPDATE` lock on the mission row plus a conditional slot claim.
///
/// ══ THE REGISTRATION WINDOW IS DERIVED, NOT READ ═══════════════════════════════════════
/// The status test reads [`EFFECTIVE_STATUS_SQL`], evaluated by Postgres against `now()` INSIDE
/// the transaction, and never the stored column — only `PATCH /events/:id` writes that column
/// and nothing moves it on a schedule, so `scheduled` and `open` — and therefore sign-ups —
/// would persist indefinitely past the operation itself. Two consequences worth stating:
///   * the window closes on the clock, not on a background task. Registration is refused
///     the first second after `start_time` whether or not the sweep has run, so the guard
///     cannot be undone by the sweeper being slow, wedged, or not deployed.
///   * reading inside the transaction also closes the check-then-claim gap where an admin
///     could cancel an event between the guard and the slot write.
///
/// ══ ONE SEAT PER CALLER ════════════════════════════════════════════════════════════════
/// A caller holds at most one `orbat_slots` row per event-mission, and it is the row their
/// `event_registrations.slot_id` names. Claiming a seat releases whatever seat the caller held
/// here first, in the same transaction — see the block above the claim. No waitlist
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
    // `map_err`, never `.ok()...unwrap_or_default()`: the latter collapses *every* extractor
    // failure — malformed JSON, a missing body, the wrong `Content-Type` — into `slot_id: ""`,
    // which is the "no seat" branch, which nulls the caller's existing claim on the way past. A
    // fat-fingered request would orphan a seat with a 200 and no diagnostic anywhere. Rejecting
    // is what every other `JsonRejection` site in this crate does.
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
    // `event_missions`. Withdraw and `assign_slot` lock `event_missions` alone. Keep it that
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

    // ══ A SEATLESS OPERATION IS NOT AN UNLIMITED ONE ══════════════════════════════════════
    // The waitlist branch below must never be spelled `capacity > 0 && registered >= capacity`.
    // That `capacity > 0` clause reads like a guard and is the opposite: at `capacity == 0` it
    // switches the capacity check OFF, so every seatless registration is accepted as
    // `registered`, without bound. Refused here, before anything is written.
    //
    // Refused rather than waitlisted, and the distinction is not cosmetic: `Waitlisted` is a
    // promise that a seat may come free, and withdraw promotes against exactly that. No seat can
    // ever come free from an ORBAT that has none, so a waitlist here would be a queue that
    // cannot move. 409 says the true thing.
    //
    // `add_event_mission` refuses to create this state at all, so reaching it means the rows
    // were seeded directly (`seeds/content_golden.sql:638`). Both doors are real: this is the
    // guard for the data, the attach refusal is the guard for the door.
    if capacity == 0 {
        return Err(ApiError::conflict(
            "this operation has no ORBAT slots, so there is nothing to register for",
        ));
    }

    // ══ `events.max_slots` — THE EVENT-WIDE ATTENDANCE CAP, ENFORCED HERE ═════════════════
    // It is validated on create (`0..=256`), editable via PATCH, and rendered by the SPA as
    // "{max_slots} slot cap" on the Event Hub header. This read is the only decision that
    // enforces it, so an operator who caps an operation at 8 and sees the cap on the page gets
    // eight. Without it the field is a promise the UI makes and nothing keeps.
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
    // above takes the `events` row `FOR UPDATE`; without that, two registrations on two different
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

    // ══ ONE SEAT PER CALLER PER OPERATION — RECONCILED HERE, UNDER THE LOCK ══════════════
    // Registering is not claim-only. Claim-only would let two *entirely valid* requests — claim
    // slot0, then claim slot1 — leave the caller holding two seats while
    // `event_registrations.slot_id` names exactly one: both seats `assigned_to` the caller, one
    // registration row, and `GET /events/:id` reporting `filled: 2, registered: 1` on a 2-slot
    // ORBAT — an operation that reads FULL with one person signed up. It takes no malformed
    // request at all to reach, which is what makes it the larger of the two orphan routes.
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
    // precisely the orphan shape above, and the caller would have to withdraw entirely to
    // unstick a seat they never meant to keep.
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
    // at or above capacity. This is the same reasoning that skips promotion when withdraw
    // releases an orphan (no registration row → never counted → nothing to promote), and
    // deliberately the same answer.
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
        // `can_manage_squad` are deliberately NOT the same predicate — here an *unreserved*
        // squad is claimable by anyone, there it is assignable only by an admin — so they share
        // the lookup (`squad_reserved_by`, which documents the faction limit) and not the
        // decision. Fixing one and missing the other is the trap; there is one query.
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
        // Zero capacity is refused above, so `capacity` is always ≥ 1 here and the bound is
        // unconditional. A `capacity > 0` guard on this comparison would not protect the check,
        // it would disable it exactly where it matters.
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
    // ══ THE SAME LOCK REGISTER TAKES ══════════════════════════════════════════════════════
    // Register serialises on this row precisely because the capacity/waitlist decision is
    // check-then-write. Withdraw does the *other* half of that same decision — it deletes a
    // registration and promotes off the waitlist — so it must queue on the same row; a lock only
    // register took would exclude nothing but other registers. Two concrete losses, both
    // reachable with valid requests:
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
    // `slot_id` is deliberately NOT selected — see the release below.
    let reg: Option<(Uuid, RegistrationState)> = sqlx::query_as(
        "SELECT id, state FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_optional(&mut *tx)
    .await?;

    // ══ RELEASE BY OCCUPANT, NOT BY THE REGISTRATION'S `slot_id` ══════════════════════════
    // Freeing the seat the *registration* points at would read the seat through a column any
    // registration write can blank, so the one state that most needs releasing — claim held,
    // `slot_id` nulled — is exactly the state it would skip. `assigned_to` is the seat claim
    // itself; it is the only column that has to be true for the seat to read as occupied, so it
    // is the one to key off.
    //
    // It is a broad delete but a bounded one — see `release_other_seats` for why `assigned_to` +
    // `event_mission_id` are the two bounds that keep it from reaching another user's claim or
    // another operation's. On healthy rows it frees exactly the seat the registration names, and
    // reaches further only on the orphans, which is the whole point. `keep: None` because a
    // withdrawal gives up everything: the caller is leaving, not moving.
    let released = release_other_seats(&mut tx, em.id, me, None).await?;

    let Some((reg_id, reg_state)) = reg else {
        // An orphaned seat — claim held, no registration row — lands here. Withdrawing means
        // "release whatever I hold here", so a caller who released something gets a 200 and
        // unsticks their own seat without an admin. A caller who genuinely holds nothing
        // released nothing, and gets the 404.
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
