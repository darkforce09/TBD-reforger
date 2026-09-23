//! Event-scoped policy references and verified membership facts used by every eligibility decision.

use std::collections::{BTreeMap, BTreeSet};

use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::event_access_policy::{EventAccessCondition, EventAccessPolicy};
use crate::operations::models::event_group::{EventGroup, EventGroupSource};

use super::evaluation::EventAccessSubject;

#[derive(Debug)]
pub struct EventAccessContext {
    pub event_id: Uuid,
    pub revision: i64,
    pub policy: EventAccessPolicy,
    pub groups: Vec<EventGroup>,
    pub squad_policies: BTreeMap<(Uuid, String, String), EventAccessPolicy>,
    pub slot_policies: BTreeMap<Uuid, EventAccessPolicy>,
}

impl EventAccessContext {
    /// Call after the event parent lock for mutations, or inside a consistent read snapshot for views.
    pub async fn load(connection: &mut PgConnection, event_id: Uuid) -> Result<Self, ApiError> {
        Self::load_many(connection, &[event_id])
            .await?
            .remove(&event_id)
            .ok_or_else(|| ApiError::not_found("event not found"))
    }

    /// Contexts of every non-deleted event in `event_ids`, in four queries. A stored policy or
    /// group that fails validation is a storage defect and never an implicit grant.
    pub async fn load_many(
        connection: &mut PgConnection,
        event_ids: &[Uuid],
    ) -> Result<BTreeMap<Uuid, Self>, ApiError> {
        let events: Vec<(Uuid, Json<EventAccessPolicy>, i64)> = sqlx::query_as(
            "SELECT id, access_policy, access_revision FROM events WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(event_ids)
        .fetch_all(&mut *connection)
        .await?;
        let group_rows: Vec<(Uuid, Uuid, String, Json<EventGroupSource>)> = sqlx::query_as(
            "SELECT id, event_id, name, source FROM event_groups
             WHERE event_id = ANY($1) AND deleted_at IS NULL ORDER BY event_id, id",
        )
        .bind(event_ids)
        .fetch_all(&mut *connection)
        .await?;
        let squad_rows: Vec<(Uuid, Uuid, String, String, Json<EventAccessPolicy>)> =
            sqlx::query_as(
                "SELECT m.event_id, p.event_mission_id, p.faction, p.squad, p.access_policy
             FROM event_squad_access_policies p JOIN event_missions m ON m.id = p.event_mission_id
             JOIN missions mission ON mission.id = m.mission_id
             WHERE m.event_id = ANY($1) AND m.deleted_at IS NULL AND mission.deleted_at IS NULL
             ORDER BY p.event_mission_id, p.faction COLLATE \"C\", p.squad COLLATE \"C\"",
            )
            .bind(event_ids)
            .fetch_all(&mut *connection)
            .await?;
        let slot_rows: Vec<(Uuid, Uuid, Option<Json<EventAccessPolicy>>)> = sqlx::query_as(
            "SELECT m.event_id, s.id, s.access_policy FROM orbat_slots s
             JOIN event_missions m ON m.id = s.event_mission_id JOIN missions mission ON mission.id = m.mission_id
             WHERE m.event_id = ANY($1) AND m.deleted_at IS NULL AND mission.deleted_at IS NULL
               AND s.access_policy IS NOT NULL ORDER BY s.id",
        )
        .bind(event_ids)
        .fetch_all(&mut *connection)
        .await?;
        let mut contexts = BTreeMap::new();
        for (event_id, policy, revision) in events {
            let context = Self {
                event_id,
                revision,
                policy: policy.0,
                groups: group_rows
                    .iter()
                    .filter(|(_, owner, _, _)| *owner == event_id)
                    .map(|(id, _, name, source)| EventGroup {
                        id: *id,
                        event_id,
                        name: name.clone(),
                        source: source.0.clone(),
                    })
                    .collect(),
                squad_policies: squad_rows
                    .iter()
                    .filter(|(owner, ..)| *owner == event_id)
                    .map(|(_, mission, faction, squad, policy)| {
                        ((*mission, faction.clone(), squad.clone()), policy.0.clone())
                    })
                    .collect(),
                slot_policies: slot_rows
                    .iter()
                    .filter(|(owner, _, _)| *owner == event_id)
                    .filter_map(|(_, slot, policy)| Some((*slot, policy.as_ref()?.0.clone())))
                    .collect(),
            };
            for group in &context.groups {
                group.source.validate().map_err(ApiError::internal)?;
            }
            for policy in std::iter::once(&context.policy)
                .chain(context.squad_policies.values())
                .chain(context.slot_policies.values())
            {
                policy.validate().map_err(ApiError::internal)?;
            }
            contexts.insert(event_id, context);
        }
        Ok(contexts)
    }

    /// Reject cross-event group references and guilds without an explicit partner group here.
    pub fn validate_policy_references(
        &self,
        policy: &EventAccessPolicy,
        main_guild: &str,
    ) -> Result<(), ApiError> {
        policy.validate().map_err(ApiError::bad_request)?;
        let guilds = self.applicable_guilds(main_guild);
        for condition in policy.grants.iter().flat_map(|grant| &grant.conditions) {
            match condition {
                EventAccessCondition::EventGroup { group_id }
                    if !self.groups.iter().any(|group| group.id == *group_id) =>
                {
                    return Err(ApiError::bad_request(
                        "policy group must belong to this event",
                    ));
                }
                EventAccessCondition::DiscordRole { guild_id, .. }
                    if !guilds.contains(guild_id) =>
                {
                    return Err(ApiError::bad_request(
                        "policy guild must be TBD or a configured partner group",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The TBD guild plus every partner guild this event's groups reference.
    pub(super) fn applicable_guilds(&self, main_guild: &str) -> BTreeSet<String> {
        let mut guilds = BTreeSet::new();
        if !main_guild.is_empty() {
            guilds.insert(main_guild.to_owned());
        }
        for group in &self.groups {
            if let EventGroupSource::PartnerGuild { guild_id, .. } = &group.source {
                guilds.insert(guild_id.clone());
            }
        }
        guilds
    }

    /// The caller locks the account before loading this projection for a reservation mutation.
    /// Transport failures preserve snapshots; expired or confirmed absent guilds contribute no grant.
    pub async fn subject(
        &self,
        connection: &mut PgConnection,
        discord_id: &str,
        main_guild: &str,
    ) -> Result<EventAccessSubject, ApiError> {
        let mut facts = self
            .account_facts(connection, &[discord_id.to_owned()], main_guild)
            .await?;
        let facts = facts
            .remove(discord_id)
            .ok_or_else(|| ApiError::not_found("account not found"))?;
        if !facts.available {
            return Err(ApiError::forbidden("account is unavailable"));
        }
        Ok(facts.current)
    }
}
