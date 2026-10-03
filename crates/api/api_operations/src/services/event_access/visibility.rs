//! Which events, attachments and seats a viewer may see.
//!
//! A viewer admitted by an event's own policy sees the whole event. A viewer admitted only by a
//! squad or slot policy discovers the event, but sees only the attachments and seats those
//! policies admit. Visibility uses current membership authority and account availability;
//! registration status, capacity and opening times never hide an event. Administrators see all.

use api_identifiers::{DiscordGuildId, DiscordUserId, EventId, OrbatSlotId};
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use super::context::EventAccessContext;
use super::evaluation::policy_admits;
use super::slot_eligibility::PolicySlot;
use super::subject_loading::{
    AccountEligibilityFacts, EventMembershipScope, build_account_facts, load_membership_snapshots,
    load_roster_entries,
};
use crate::models::event_group::EventGroupSource;
use api_foundation::error_handling::api_error::ApiError;

/// How much of one event a viewer may see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventVisibility {
    /// The viewer may not discover the event.
    Hidden,
    /// Admitted only through child policies: exactly these seats may be shown.
    Partial {
        /// Seats whose slot or squad policy admits the viewer.
        admitted_slots: BTreeSet<OrbatSlotId>,
    },
    /// The event's own policy admits the viewer, or the viewer is an administrator.
    Full,
}

impl EventVisibility {
    /// Whether the viewer may discover the event at all.
    pub fn is_visible(&self) -> bool {
        !matches!(self, Self::Hidden)
    }

    /// Whether a seat of a visible event may be shown to the viewer.
    pub fn shows_slot(&self, slot: OrbatSlotId) -> bool {
        match self {
            Self::Hidden => false,
            Self::Partial { admitted_slots } => admitted_slots.contains(&slot),
            Self::Full => true,
        }
    }
}

/// The viewer's per-event facts, for eligibility displays beside visibility.
pub struct ViewerEventAccess {
    /// How much of the event the viewer may see.
    pub visibility: EventVisibility,
    /// The event's validated access policies and groups.
    pub context: EventAccessContext,
    /// The viewer's membership facts for the event.
    pub facts: AccountEligibilityFacts,
}

/// One seat of an operational attachment, with the event it belongs to.
#[derive(sqlx::FromRow)]
struct EventSeat {
    event_id: EventId,
    #[sqlx(flatten)]
    slot: PolicySlot,
}

/// Seats of the operational attachments of `events`, keyed by event.
async fn load_event_slots(
    connection: &mut PgConnection,
    events: &[EventId],
) -> Result<BTreeMap<EventId, Vec<PolicySlot>>, ApiError> {
    let rows: Vec<EventSeat> = sqlx::query_as(
        "SELECT m.event_id, s.id, s.event_mission_id, s.faction, s.squad, s.slot_index, s.assigned_to
         FROM orbat_slots s JOIN event_missions m ON m.id = s.event_mission_id
         WHERE m.event_id = ANY($1) AND m.deleted_at IS NULL
         ORDER BY s.faction COLLATE \"C\", s.squad COLLATE \"C\", s.slot_index, s.id",
    )
    .bind(events)
    .fetch_all(connection)
    .await?;
    let mut slots: BTreeMap<EventId, Vec<PolicySlot>> = BTreeMap::new();
    for seat in rows {
        slots.entry(seat.event_id).or_default().push(seat.slot);
    }
    Ok(slots)
}

/// Visibility and current facts of one viewer for each of `events`, in a handful of queries.
/// Events that do not exist or are deleted are absent from the result.
pub async fn viewer_event_access(
    connection: &mut PgConnection,
    viewer: &DiscordUserId,
    is_administrator: bool,
    events: &[EventId],
    main_guild: &DiscordGuildId,
) -> Result<BTreeMap<EventId, ViewerEventAccess>, ApiError> {
    let contexts = EventAccessContext::load_many(connection, events).await?;
    let slots = load_event_slots(connection, events).await?;
    let account: Option<(bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT NOT is_banned AND deleted_at IS NULL, clock_timestamp() FROM users WHERE discord_id = $1",
    )
    .bind(viewer)
    .fetch_optional(&mut *connection)
    .await?;
    let (available, now) = account.ok_or_else(|| ApiError::unauthorized("invalid session"))?;
    let mut guilds: BTreeSet<String> = BTreeSet::new();
    if !main_guild.as_str().is_empty() {
        guilds.insert(main_guild.to_string());
    }
    for context in contexts.values() {
        for group in &context.groups {
            if let EventGroupSource::PartnerGuild { guild_id, .. } = &group.source {
                guilds.insert(guild_id.clone());
            }
        }
    }
    let accounts = [viewer.to_owned()];
    let guild_list: Vec<String> = guilds.into_iter().collect();
    let snapshots = load_membership_snapshots(connection, &accounts, &guild_list).await?;
    let roster = load_roster_entries(connection, events, &accounts).await?;
    let mut access = BTreeMap::new();
    for (event, context) in contexts {
        let applicable = context.applicable_guilds(main_guild);
        let facts = build_account_facts(
            viewer,
            available,
            now,
            snapshots.iter(),
            roster.iter().filter(|entry| entry.event_id == event),
            &EventMembershipScope {
                groups: &context.groups,
                applicable_guilds: &applicable,
                main_guild,
            },
        );
        let visibility = if is_administrator {
            EventVisibility::Full
        } else if !available {
            EventVisibility::Hidden
        } else if policy_admits(&context.policy, &facts.current) {
            EventVisibility::Full
        } else {
            let admitted_slots: BTreeSet<OrbatSlotId> = slots
                .get(&event)
                .into_iter()
                .flatten()
                .filter(|slot| context.slot_admits(slot, &facts.current))
                .map(|slot| slot.id)
                .collect();
            if admitted_slots.is_empty() {
                EventVisibility::Hidden
            } else {
                EventVisibility::Partial { admitted_slots }
            }
        };
        access.insert(
            event,
            ViewerEventAccess {
                visibility,
                context,
                facts,
            },
        );
    }
    Ok(access)
}
