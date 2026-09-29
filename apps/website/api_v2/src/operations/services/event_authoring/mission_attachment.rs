//! Attaching a mission to an event: the ORBAT the attachment seats, and the transaction body that
//! snapshots it into `orbat_slots` rows under the new `event_missions` row.
//!
//! **Role:** the only writer of `orbat_slots` through the API. [`AttachmentTemplate`] resolves the
//! ORBAT an attachment seats and refuses one that is unreadable, seats nobody, or names a faction
//! no armory line could match; [`attach_mission`] takes the locks, restores or inserts the
//! attachment, materializes its slots and appends the audit row.
//!
//! **Position:** `POST /api/v1/events/:id/missions`
//! ([`add_event_mission`](crate::operations::handlers::event_mission_attachment::add_event_mission))
//! resolves the template from the request or the mission's published version and attaches for an
//! administrator session; the `staging-fixtures seed-load-fixture-events` host tool attaches its
//! fixture ORBAT for its reserved author. Both run [`attach_mission`] on their own transaction.
//!
//! **Signals & state:** none; [`attach_mission`] runs on the caller's transaction.
//!
//! **Invariants:** an [`AttachmentTemplate`] seats at least one slot and every squad's faction is
//! non-empty and equal to its trimmed form, and it is the only template [`attach_mission`] takes;
//! the attach locks the event, then the mission (the order the catalog lifecycle guard holds),
//! then the event scope with its authority check; a deleted or archived mission is never attached;
//! a second attachment of a mission still on the event is a 409; a removed attachment of the same
//! mission is restored with its retained ORBAT and signup history; slot rows store `faction` byte
//! for byte; the attachment and its audit row commit together or not at all.

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;
use website_map_engine::data::scenario::orbat::validate_faction_join_key;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::configuration::Config;
use crate::core::database::postgres_errors::is_unique_violation;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::operations::models::{EventMission, EventStatus};
use crate::operations::services::event_lifecycle_transition::advance_locked_event;
use crate::operations::services::event_reservations::event_administration::{
    lock_event_scope, normalize_schedule_time,
};
use crate::operations::services::event_reservations::mission_restoration::restore_if_removed;
use crate::operations::services::event_reservations::reservation_scope::{
    AttachmentScope, ReservationScope,
};
use crate::operations::services::{OrbatSquadTemplate, parse_orbat_template};

/// Who an attachment is recorded for, and how its authority is settled inside the attachment's
/// transaction.
#[derive(Clone, Copy)]
pub enum AttachmentAuthority<'a> {
    /// A signed-in administrator. The event scope lock rechecks the session after its lock wait
    /// and refuses an account that no longer holds the administrator role.
    AdministratorSession {
        /// The session's account.
        administrator: &'a AuthUser,
        /// The configuration the session is rechecked against.
        config: &'a Config,
    },
    /// An account a host tool acts for, whose authority the tool's own guards settled: the event
    /// scope is locked as a system transaction, with no session to recheck.
    HostToolAccount {
        /// The Discord id the audit row names; an account must hold it.
        discord_id: &'a str,
    },
}

impl AttachmentAuthority<'_> {
    /// The Discord id the attachment's audit row names.
    fn audit_actor(&self) -> &str {
        match self {
            Self::AdministratorSession { administrator, .. } => &administrator.discord_id,
            Self::HostToolAccount { discord_id } => discord_id,
        }
    }

    /// Lock the event scope, recheck the session where one carries the authority, and store any
    /// automatic lifecycle move the event is due before the attachment changes its schedule.
    async fn lock_scope(
        &self,
        connection: &mut PgConnection,
        event_id: Uuid,
    ) -> Result<(), ApiError> {
        match *self {
            Self::AdministratorSession {
                administrator,
                config,
            } => {
                lock_event_scope(connection, event_id, administrator, config).await?;
            }
            Self::HostToolAccount { .. } => {
                let scope = ReservationScope::lock(
                    connection,
                    event_id,
                    AttachmentScope::IncludingRemoved,
                    None,
                    &[],
                )
                .await?;
                let stored: EventStatus =
                    sqlx::query_scalar("SELECT status FROM events WHERE id = $1")
                        .bind(event_id)
                        .fetch_one(&mut *connection)
                        .await?;
                advance_locked_event(connection, event_id, stored, scope.event.status).await?;
            }
        }
        Ok(())
    }
}

/// Where an attachment's ORBAT came from; it decides only the wording of a zero-slot refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplateSource {
    /// The caller's explicit `orbat`.
    Request,
    /// The mission's current published version.
    PublishedVersion,
}

/// An ORBAT an attachment may seat: at least one slot, and every squad's faction a join key an
/// armory line can match. It is built only through those checks.
#[derive(Debug, Clone)]
pub struct AttachmentTemplate {
    squads: Vec<OrbatSquadTemplate>,
}

impl AttachmentTemplate {
    /// The template an attachment of `mission_id` seats: `requested` when it holds any squad, else
    /// the ORBAT of the mission's current published version.
    pub async fn resolve(
        pool: &PgPool,
        mission_id: Uuid,
        requested: Vec<OrbatSquadTemplate>,
    ) -> Result<Self, ApiError> {
        if requested.is_empty() {
            let published = orbat_template_for_mission(pool, mission_id).await?;
            Self::checked(published, TemplateSource::PublishedVersion)
        } else {
            Self::checked(requested, TemplateSource::Request)
        }
    }

    /// An explicit template, refused when it seats nobody (an empty list included) or when a
    /// squad's faction is not a usable join key.
    pub fn requested(squads: Vec<OrbatSquadTemplate>) -> Result<Self, ApiError> {
        Self::checked(squads, TemplateSource::Request)
    }

    /// The number of slots an attachment of this template materializes.
    pub fn slot_count(&self) -> usize {
        self.squads.iter().map(|squad| squad.slots.len()).sum()
    }

    fn checked(squads: Vec<OrbatSquadTemplate>, source: TemplateSource) -> Result<Self, ApiError> {
        // ══ THE ZERO-SLOT REFUSAL — UNLIMITED SEATLESS REGISTRATION STARTS HERE ═══════════════
        // `materialize_slots` is the ONLY producer of `orbat_slots` through this API, and a
        // template with no slots makes it write nothing while the transaction still commits and
        // still answers 201. Capacity in `register_for_event_mission` is `count(orbat_slots)`, so
        // it lands on 0 — and a guard spelled `capacity > 0 && registered >= capacity` is, at 0,
        // simply off. Unlimited registration onto an event that can seat nobody.
        //
        // Refusing rather than attaching-but-not-registerable, because there is no way back:
        // nothing else inserts `orbat_slots` (`assign_slot` / `clear_slot` only move
        // `assigned_to`; the only other writer in the tree is the dev seed), so a zero-slot
        // `event_missions` row can NEVER gain slots, and the sole recovery is a detach and a
        // re-attach that an administrator has to know to do, having just been told the attach
        // succeeded. The ORBAT is authored upstream, on the mission, and an attach snapshots it.
        //
        // The count is over SLOTS, not squads: a non-empty squad list whose `slots` arrays are
        // all empty is the same zero-row outcome from a perfectly *valid* payload.
        if squads.iter().all(|sq| sq.slots.is_empty()) {
            return Err(match source {
                TemplateSource::Request => ApiError::bad_request(
                    "`orbat` describes no slots, so nobody could be seated — every squad's `slots` array is empty",
                ),
                TemplateSource::PublishedVersion => ApiError::conflict(
                    "this mission's ORBAT describes no slots, so nobody could be seated — author its ORBAT and publish a version, or attach with an explicit `orbat`",
                ),
            });
        }

        // ══ ORBAT `faction` IS THE EVENT HUB JOIN KEY ════════════════════════════════════════
        // Same shape as armory `faction`: require non-empty-after-trim; refuse a value that
        // differs from its trimmed form; store verbatim in `materialize_slots`. Both doors into
        // that writer (request `orbat` and mission-payload derive) pass here, so one check closes
        // both. A whitespace-only ORBAT faction is permanently unmatchable against the armory.
        for (i, sq) in squads.iter().enumerate() {
            if let Err(msg) = validate_faction_join_key(&sq.faction) {
                return Err(ApiError::bad_request(format!("orbat[{i}].{msg}")));
            }
        }
        Ok(Self { squads })
    }
}

/// Resolve a mission's ORBAT template from its current published version payload.
///
/// Each failure is a different sentence, and only one of the four is the caller's to fix:
///
///   * **mission row gone** → 404. A delete racing the attach; the caller already checked.
///   * **no published version** → 409. The mission is real; it is the mission's *state* that
///     cannot satisfy the request. Recoverable by publishing a version.
///   * **`current_version_id` dangling** → 500, logged. `missions.current_version_id` carries
///     **no foreign key**, so a mission can name a version row that does not exist. Nobody
///     outside can fix that and it must not be quiet.
///   * **unreadable `orbat`** → 400 naming the payload, in [`template_from_payload`].
///
/// A zero-slot *success* is not decided here; see the refusal in `AttachmentTemplate::checked`,
/// the one place both doors into `materialize_slots` meet.
async fn orbat_template_for_mission(
    pool: &PgPool,
    mission_id: Uuid,
) -> Result<Vec<OrbatSquadTemplate>, ApiError> {
    let cur: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT current_version_id FROM missions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(mission_id)
    .fetch_optional(pool)
    .await?;
    let Some(current_version_id) = cur else {
        return Err(ApiError::not_found("mission not found"));
    };
    let Some(vid) = current_version_id else {
        return Err(ApiError::conflict(
            "this mission has no published version, so it has no ORBAT to seat — publish a version, or attach with an explicit `orbat`",
        ));
    };
    let payload: Option<crate::core::wire_format::RawJson> =
        sqlx::query_scalar("SELECT json_payload FROM mission_versions WHERE id = $1")
            .bind(vid)
            .fetch_optional(pool)
            .await?;
    let Some(payload) = payload else {
        tracing::error!(
            %mission_id,
            version_id = %vid,
            "missions.current_version_id names a mission_versions row that does not exist"
        );
        return Err(ApiError::internal(
            "this mission's published version is missing from the database",
        ));
    };
    template_from_payload(payload.0.get().as_bytes())
}

/// [`parse_orbat_template`] with its one silent failure made loud.
///
/// `parse_orbat_template` lives in the shared map-engine crate and returns a bare `Vec`, so it
/// falls back to editor-derived squads when the explicit list is absent or empty:
///
/// ```rust
/// use website_api::operations::services::parse_orbat_template;
/// let squads = parse_orbat_template(br#"{"orbat":[]}"#);
/// assert!(squads.is_empty());
/// ```
///
/// An explicit top-level `orbat[]` that fails to deserialize therefore does not merely vanish —
/// it is **replaced** by the editor-derived ORBAT, a *different and possibly non-empty* seating
/// plan. This function mirrors that precedence exactly and differs only there:
///   * `orbat` absent, `null`, or the payload not an object → fall through verbatim;
///   * `orbat` present, deserializable, but **empty** → fall through, as the parser does;
///   * `orbat` present, deserializable, non-empty → return it, as the parser does;
///   * `orbat` present and NOT deserializable → 400, carrying serde's own message in `details`.
fn template_from_payload(payload: &[u8]) -> Result<Vec<OrbatSquadTemplate>, ApiError> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Probe {
        orbat: Option<Value>,
    }
    if let Ok(Probe { orbat: Some(orbat) }) = serde_json::from_slice::<Probe>(payload)
        && !orbat.is_null()
    {
        match serde_json::from_value::<Vec<OrbatSquadTemplate>>(orbat) {
            Ok(squads) if !squads.is_empty() => return Ok(squads),
            Ok(_) => {}
            Err(e) => {
                return Err(ApiError::with_details(
                    StatusCode::BAD_REQUEST,
                    "this mission's published version has an `orbat` this API cannot read",
                    json!({ "orbat": e.to_string() }),
                ));
            }
        }
    }
    Ok(parse_orbat_template(payload))
}

/// Attach `mission_id` to `event_id` at `start_time`, seating `template`, on the caller's
/// transaction, and append the audit row naming `authority`'s account.
///
/// Restores an earlier removed attachment of the same mission with its retained ORBAT and signup
/// history (`event.mission_restored`); otherwise inserts the event mission and one slot per
/// template slot (`event.mission_attached`). Refuses a missing event or mission (404), an archived
/// mission (409) and a mission already attached to the event (409): `idx_event_mission` is unique
/// on `(event_id, mission_id)`, and its violation maps to that 409 rather than a 500.
pub async fn attach_mission(
    connection: &mut PgConnection,
    event_id: Uuid,
    mission_id: Uuid,
    start_time: DateTime<Utc>,
    template: &AttachmentTemplate,
    authority: &AttachmentAuthority<'_>,
) -> Result<EventMission, ApiError> {
    let start_time = normalize_schedule_time(start_time)?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM events WHERE id = $1 AND deleted_at IS NULL FOR NO KEY UPDATE",
    )
    .bind(event_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("event not found"))?;
    // The catalog lifecycle guard holds this same lock before checking active attachments.
    let current_mission: Option<(bool, String)> = sqlx::query_as(
        "SELECT deleted_at IS NOT NULL, status::text FROM missions WHERE id = $1 FOR NO KEY UPDATE",
    )
    .bind(mission_id)
    .fetch_optional(&mut *connection)
    .await?;
    match current_mission {
        None | Some((true, _)) => return Err(ApiError::not_found("mission not found")),
        Some((false, status)) if status == "archived" => {
            return Err(ApiError::conflict(
                "an archived mission cannot be attached; restore its lifecycle state first",
            ));
        }
        _ => {}
    }
    authority.lock_scope(connection, event_id).await?;
    if let Some(restored) = restore_if_removed(
        connection,
        event_id,
        mission_id,
        start_time,
        &template.squads,
    )
    .await?
    {
        append_actor_audit(
            connection,
            authority.audit_actor(),
            "event.mission_restored",
            "event",
            &event_id.to_string(),
            "Removed mission restored with retained ORBAT and signup history",
        )
        .await?;
        return Ok(restored);
    }
    let em: EventMission = match sqlx::query_as(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, $3, now(), now()) RETURNING id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at",
    )
    .bind(event_id)
    .bind(mission_id)
    .bind(start_time)
    .fetch_one(&mut *connection)
    .await
    {
        Ok(em) => em,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "this mission is already attached to this event",
            ));
        }
        Err(e) => return Err(e.into()),
    };
    materialize_slots(connection, em.id, &template.squads).await?;
    append_actor_audit(
        connection,
        authority.audit_actor(),
        "event.mission_attached",
        "event",
        &event_id.to_string(),
        "Mission and ORBAT attached",
    )
    .await?;
    Ok(em)
}

/// Materialize parsed squads into OrbatSlot rows for one event mission.
///
/// **`faction` is stored verbatim.** [`AttachmentTemplate`] has already refused empty,
/// whitespace-only and padded values via [`validate_faction_join_key`]. Do not trim here:
/// `orbat_slots.faction` is matched byte-for-byte against `mission_armories.faction`, and
/// canonicalising one side of that join is what makes a faction unmatchable.
async fn materialize_slots(
    tx: &mut sqlx::PgConnection,
    em_id: Uuid,
    squads: &[OrbatSquadTemplate],
) -> sqlx::Result<()> {
    for sq in squads {
        for (i, sl) in sq.slots.iter().enumerate() {
            sqlx::query(
                "INSERT INTO orbat_slots (event_mission_id, faction, callsign, squad, role, loadout, tag, slot_index) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(em_id)
            .bind(&sq.faction)
            .bind(&sq.callsign)
            .bind(&sq.squad)
            .bind(&sl.role)
            .bind(&sl.loadout)
            .bind(&sl.tag)
            .bind(i as i64)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/mission_attachment.rs"]
mod tests;
