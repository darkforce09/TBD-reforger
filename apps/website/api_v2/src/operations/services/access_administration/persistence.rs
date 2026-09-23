//! Persistence for manager-controlled policies, groups and quotas, and the access view.
//!
//! Callers hold the administrator event scope. Every change first checks the access revision the
//! manager prepared it against and then advances it, so a stale form cannot overwrite a newer one.

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::Event;
use crate::operations::models::event_access_administration::{
    AuthorshipProvenance, EventAccessAdministration, EventGroupView, QuotaUsageView,
    RosterEntryView, SlotAccessPolicy, SquadAccessPolicy,
};
use crate::operations::models::event_access_policy::{EventAccessCondition, EventAccessPolicy};
use crate::operations::models::event_group::EventGroupSource;
use crate::operations::models::reservation_quota::{ReservationQuotaKind, ReservationQuotas};
use crate::operations::services::event_access::context::EventAccessContext;
use crate::operations::services::event_reservations::event_administration::allocation_usage;
use crate::operations::services::event_reservations::participant_allocations::load_reservation_quotas;

/// Reject a change prepared against an older access revision, then advance the revision.
pub async fn advance_access_revision(
    connection: &mut PgConnection,
    event_id: Uuid,
    expected: i64,
) -> Result<i64, ApiError> {
    let current: i64 = sqlx::query_scalar("SELECT access_revision FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_one(&mut *connection)
        .await?;
    if current != expected {
        return Err(ApiError::with_details(
            StatusCode::CONFLICT,
            "access settings changed since this form was loaded; reload and apply the change again",
            json!({"code": "ACCESS_REVISION_CONFLICT", "access_revision": current}),
        ));
    }
    Ok(sqlx::query_scalar(
        "UPDATE events SET access_revision = access_revision + 1, updated_at = now()
         WHERE id = $1 RETURNING access_revision",
    )
    .bind(event_id)
    .fetch_one(connection)
    .await?)
}

/// Policies must be well formed and may only name this event's groups and applicable guilds.
pub async fn validate_policy(
    connection: &mut PgConnection,
    event_id: Uuid,
    policy: &EventAccessPolicy,
    main_guild: &str,
) -> Result<(), ApiError> {
    EventAccessContext::load(connection, event_id)
        .await?
        .validate_policy_references(policy, main_guild)
}

pub async fn store_event_policy(
    connection: &mut PgConnection,
    event_id: Uuid,
    policy: &EventAccessPolicy,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE events SET access_policy = $2 WHERE id = $1")
        .bind(event_id)
        .bind(Json(policy))
        .execute(connection)
        .await?;
    Ok(())
}

/// `None` removes the squad's explicit policy so it inherits the event policy again.
pub async fn store_squad_policy(
    connection: &mut PgConnection,
    mission: Uuid,
    faction: &str,
    squad: &str,
    policy: Option<&EventAccessPolicy>,
) -> Result<(), ApiError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM orbat_slots WHERE event_mission_id = $1 AND faction = $2 AND squad = $3)",
    )
    .bind(mission)
    .bind(faction)
    .bind(squad)
    .fetch_one(&mut *connection)
    .await?;
    if !exists {
        return Err(ApiError::not_found("squad not found in this ORBAT"));
    }
    match policy {
        Some(policy) => sqlx::query(
            "INSERT INTO event_squad_access_policies (event_mission_id, faction, squad, access_policy)
             VALUES ($1, $2, $3, $4) ON CONFLICT (event_mission_id, faction, squad)
             DO UPDATE SET access_policy = EXCLUDED.access_policy",
        )
        .bind(mission)
        .bind(faction)
        .bind(squad)
        .bind(Json(policy))
        .execute(connection)
        .await?,
        None => sqlx::query(
            "DELETE FROM event_squad_access_policies
             WHERE event_mission_id = $1 AND faction = $2 AND squad = $3",
        )
        .bind(mission)
        .bind(faction)
        .bind(squad)
        .execute(connection)
        .await?,
    };
    Ok(())
}

/// `None` removes the slot's explicit policy so it inherits its squad or event policy again.
pub async fn store_slot_policy(
    connection: &mut PgConnection,
    mission: Uuid,
    slot: Uuid,
    policy: Option<&EventAccessPolicy>,
) -> Result<(), ApiError> {
    let updated = sqlx::query(
        "UPDATE orbat_slots SET access_policy = $3 WHERE id = $1 AND event_mission_id = $2",
    )
    .bind(slot)
    .bind(mission)
    .bind(policy.map(Json))
    .execute(connection)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::not_found("slot not found"));
    }
    Ok(())
}

/// A group still named by any policy cannot be removed; managers edit the policy first.
pub fn group_is_referenced(context: &EventAccessContext, group: Uuid) -> bool {
    std::iter::once(&context.policy)
        .chain(context.squad_policies.values())
        .chain(context.slot_policies.values())
        .flat_map(|policy| policy.grants.iter())
        .flat_map(|grant| grant.conditions.iter())
        .any(|condition| matches!(condition, EventAccessCondition::EventGroup { group_id } if *group_id == group))
}

pub fn validate_group(name: &str, source: &EventGroupSource) -> Result<(), ApiError> {
    if name.trim().is_empty() || name.trim() != name || name.len() > 128 {
        return Err(ApiError::bad_request(
            "group name must contain 1 to 128 bytes without surrounding whitespace",
        ));
    }
    source.validate().map_err(ApiError::bad_request)
}

/// Pools replace their stored values; the caller validates them against current allocations.
pub async fn store_reservation_quotas(
    connection: &mut PgConnection,
    event_id: Uuid,
    quotas: &ReservationQuotas,
) -> Result<(), ApiError> {
    for kind in [
        ReservationQuotaKind::Member,
        ReservationQuotaKind::Guest,
        ReservationQuotaKind::Open,
    ] {
        let pool = quotas.pool(kind);
        sqlx::query(
            "UPDATE event_reservation_quota_pools SET seat_limit = $3, opens_at = $4
             WHERE event_id = $1 AND quota_kind = $2",
        )
        .bind(event_id)
        .bind(kind.as_str())
        .bind(pool.seats.map(i64::from))
        .bind(pool.opens_at)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct GroupRow {
    id: Uuid,
    name: String,
    source: Json<EventGroupSource>,
    created_by: Option<String>,
    system_origin: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RosterRow {
    group_id: Uuid,
    discord_id: String,
    username: String,
    added_by: Option<String>,
    system_origin: Option<String>,
    added_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SlotPolicyRow {
    id: Uuid,
    event_mission_id: Uuid,
    faction: String,
    squad: String,
    slot_index: i64,
    /// Filtered to explicit policies; `None` never reaches the view.
    access_policy: Option<Json<EventAccessPolicy>>,
}

/// The complete manager view, read inside the caller's transaction or snapshot.
pub async fn load_access_administration(
    connection: &mut PgConnection,
    event: &Event,
    active_missions: &[Uuid],
) -> Result<EventAccessAdministration, ApiError> {
    let event_id = event.id;
    let context = EventAccessContext::load(connection, event_id).await?;
    let slot_rows: Vec<SlotPolicyRow> = sqlx::query_as(
        "SELECT id, event_mission_id, faction, squad, slot_index, access_policy FROM orbat_slots
         WHERE event_mission_id = ANY($1) AND access_policy IS NOT NULL
         ORDER BY event_mission_id, faction COLLATE \"C\", squad COLLATE \"C\", slot_index, id",
    )
    .bind(active_missions)
    .fetch_all(&mut *connection)
    .await?;
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, name, source, created_by, system_origin, created_at FROM event_groups
         WHERE event_id = $1 AND deleted_at IS NULL ORDER BY created_at, id",
    )
    .bind(event_id)
    .fetch_all(&mut *connection)
    .await?;
    let roster: Vec<RosterRow> = sqlx::query_as(
        "SELECT r.group_id, r.discord_id, COALESCE(u.username, '') AS username, r.added_by,
             r.system_origin, r.added_at
         FROM event_group_roster r JOIN event_groups g ON g.id = r.group_id
         JOIN users u ON u.discord_id = r.discord_id
         WHERE g.event_id = $1 AND g.deleted_at IS NULL AND r.removed_at IS NULL
         ORDER BY r.added_at, r.discord_id COLLATE \"C\"",
    )
    .bind(event_id)
    .fetch_all(&mut *connection)
    .await?;
    let usage = allocation_usage(connection, event_id, active_missions).await?;
    let reservation_quotas = load_reservation_quotas(connection, event_id).await?;
    Ok(EventAccessAdministration {
        event_id,
        access_revision: context.revision,
        max_slots: event.max_slots,
        event_policy: context.policy.clone(),
        squad_policies: context
            .squad_policies
            .iter()
            .map(|((mission, faction, squad), policy)| SquadAccessPolicy {
                event_mission_id: *mission,
                faction: faction.clone(),
                squad: squad.clone(),
                policy: policy.clone(),
            })
            .collect(),
        slot_policies: slot_rows
            .into_iter()
            .filter_map(|row| {
                Some(SlotAccessPolicy {
                    policy: row.access_policy?.0,
                    slot_id: row.id,
                    event_mission_id: row.event_mission_id,
                    faction: row.faction,
                    squad: row.squad,
                    slot_index: row.slot_index,
                })
            })
            .collect(),
        groups: groups
            .into_iter()
            .map(|group| EventGroupView {
                roster: roster
                    .iter()
                    .filter(|entry| entry.group_id == group.id)
                    .map(|entry| RosterEntryView {
                        discord_id: entry.discord_id.clone(),
                        username: entry.username.clone(),
                        added_by: entry.added_by.clone(),
                        system_origin: entry.system_origin.clone(),
                        added_at: entry.added_at,
                    })
                    .collect(),
                id: group.id,
                name: group.name,
                source: group.source.0,
                provenance: AuthorshipProvenance {
                    created_by: group.created_by,
                    system_origin: group.system_origin,
                    created_at: group.created_at,
                },
            })
            .collect(),
        reservation_quotas,
        quota_usage: QuotaUsageView {
            member: usage.member,
            guest: usage.guest,
            open: usage.open,
            legacy_unclassified: usage.legacy_unclassified,
            total: usage.total().map_err(ApiError::internal)?,
        },
    })
}
