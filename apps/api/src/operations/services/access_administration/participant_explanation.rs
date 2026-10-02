//! Why each participant of an event is, or is not, admitted: the effective policy, the grants
//! that hold, and the provenance of every membership fact those grants rely on.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::RegistrationState;
use crate::operations::models::event_access_administration::{
    GuildEvidenceView, ParticipantAccessExplanation, ParticipantAllocationView,
    RegistrationDecision, RosterEvidenceView,
};
use crate::operations::models::participant_allocation::ParticipantAllocationKind;
use crate::operations::services::event_access::context::EventAccessContext;
use crate::operations::services::event_access::evaluation::{PolicySource, admitting_grants};
use crate::operations::services::event_access::slot_eligibility::load_policy_slots;

#[derive(sqlx::FromRow)]
struct RegistrationRow {
    id: Uuid,
    discord_id: String,
    event_mission_id: Uuid,
    reservation_state: RegistrationState,
    slot_id: Option<Uuid>,
    queue_entered_at: DateTime<Utc>,
}

/// Explain every account with a current reservation, waiting entry, seat or allocation.
/// Read inside one snapshot so every fact belongs to the same instant.
pub async fn explain_participants(
    connection: &mut PgConnection,
    event_id: Uuid,
    active_missions: &[Uuid],
    main_guild: &str,
) -> Result<Vec<ParticipantAccessExplanation>, ApiError> {
    let context = EventAccessContext::load(connection, event_id).await?;
    let slots = load_policy_slots(connection, active_missions).await?;
    let registrations: Vec<RegistrationRow> = sqlx::query_as(
        "SELECT id, discord_id, event_mission_id, reservation_state, slot_id, queue_entered_at
         FROM event_registrations WHERE event_mission_id = ANY($1)
           AND reservation_state IN ('registered', 'legacy_unknown', 'waitlisted')
         ORDER BY queue_entered_at, id",
    )
    .bind(active_missions)
    .fetch_all(&mut *connection)
    .await?;
    let allocations: Vec<(String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT discord_id, quota_kind, acquired_at FROM event_participant_allocations
         WHERE event_id = $1 AND released_at IS NULL",
    )
    .bind(event_id)
    .fetch_all(&mut *connection)
    .await?;
    let mut accounts: Vec<String> = registrations
        .iter()
        .map(|row| row.discord_id.clone())
        .chain(slots.iter().filter_map(|slot| slot.assigned_to.clone()))
        .chain(allocations.iter().map(|(account, _, _)| account.clone()))
        .collect();
    accounts.sort_unstable();
    accounts.dedup();
    let names: BTreeMap<String, String> = sqlx::query_as(
        "SELECT discord_id, COALESCE(username, '') FROM users WHERE discord_id = ANY($1)",
    )
    .bind(&accounts)
    .fetch_all(&mut *connection)
    .await?
    .into_iter()
    .collect();
    let facts = context
        .account_facts(connection, &accounts, main_guild)
        .await?;
    let mut explanations = Vec::with_capacity(accounts.len());
    for account in accounts {
        let Some(account_facts) = facts.get(&account) else {
            continue;
        };
        let allocation = allocations
            .iter()
            .find(|(owner, _, _)| *owner == account)
            .map(|(_, kind, acquired_at)| {
                ParticipantAllocationKind::parse(kind)
                    .map(|quota_kind| ParticipantAllocationView {
                        quota_kind,
                        acquired_at: *acquired_at,
                    })
                    .ok_or_else(|| ApiError::internal("stored allocation kind is unknown"))
            })
            .transpose()?;
        let decisions = registrations
            .iter()
            .filter(|row| row.discord_id == account)
            .map(|row| {
                let seat = row
                    .slot_id
                    .and_then(|id| slots.iter().find(|slot| slot.id == id));
                let (policy_source, grants, current, verified) = match seat {
                    Some(slot) => {
                        let (policy, source) = context.effective_slot_policy(slot);
                        (
                            source,
                            admitting_grants(policy, &account_facts.current),
                            context.slot_admits(slot, &account_facts.current),
                            context.slot_admits(slot, &account_facts.last_verified),
                        )
                    }
                    None => {
                        let mission_slots = slots
                            .iter()
                            .filter(|slot| slot.event_mission_id == row.event_mission_id);
                        let mut current = false;
                        let mut verified = false;
                        for slot in mission_slots {
                            current |= context.slot_admits(slot, &account_facts.current);
                            verified |= context.slot_admits(slot, &account_facts.last_verified);
                        }
                        (
                            PolicySource::Event,
                            admitting_grants(&context.policy, &account_facts.current),
                            current,
                            verified,
                        )
                    }
                };
                RegistrationDecision {
                    registration_id: row.id,
                    event_mission_id: row.event_mission_id,
                    reservation_state: row.reservation_state,
                    slot_id: row.slot_id,
                    queue_entered_at: row.queue_entered_at,
                    policy_source,
                    admitting_grants: grants,
                    current_authority_admits: current,
                    last_verified_admits: verified,
                }
            })
            .collect();
        explanations.push(ParticipantAccessExplanation {
            username: names.get(&account).cloned().unwrap_or_default(),
            available: account_facts.available,
            tbd_member: account_facts.current.tbd_member,
            // Every guild the event's policies rely on is listed; one never observed for this
            // account is unknown, which is what makes its verification visibly pending.
            guilds: account_facts
                .guilds
                .iter()
                .map(|(guild_id, evidence)| GuildEvidenceView {
                    guild_id: guild_id.clone(),
                    membership_status: evidence.membership_status.clone(),
                    verified_at: evidence.verified_at,
                    override_until: evidence.override_until,
                    current: evidence.current,
                })
                .chain(
                    account_facts
                        .unverified_guilds
                        .iter()
                        .filter(|guild_id| !account_facts.guilds.contains_key(*guild_id))
                        .map(|guild_id| GuildEvidenceView {
                            guild_id: guild_id.clone(),
                            membership_status: "unknown".to_owned(),
                            verified_at: None,
                            override_until: None,
                            current: false,
                        }),
                )
                .collect(),
            roster_groups: account_facts
                .roster_groups
                .iter()
                .map(|(group_id, entry)| RosterEvidenceView {
                    group_id: *group_id,
                    added_by: entry.added_by.clone(),
                    system_origin: entry.system_origin.clone(),
                    added_at: entry.added_at,
                })
                .collect(),
            allocation,
            registrations: decisions,
            discord_id: account,
        });
    }
    Ok(explanations)
}
