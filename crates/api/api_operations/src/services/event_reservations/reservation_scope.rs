//! The single lock order every reservation writer uses:
//! event → attachments in UUID order → complete sorted account union → authority recheck.
//!
//! The account union is collected while the parent locks are held, so no registration, seat or
//! allocation of the event can gain an account outside it before the transaction ends. It covers
//! registrants in every state (including every waiting candidate a promotion may inspect), seat
//! occupants, active allocation holders, the actor and any explicitly named account. Promotion and
//! re-evaluation never lock further accounts, so they cannot invert account and event locks.

use api_identifiers::{DiscordUserId, EventId, EventMissionId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::event_administration::load_locked_event;
use crate::models::Event;
use api_caller_identity::identity_ownership::lock_accounts;
use api_caller_identity::session_authorization::authorize_on_connection;
use api_configuration::configuration::Config;
use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::middleware::AuthUser;

/// Which attachments the transaction may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentScope {
    /// Reservation changes touch operational attachments only.
    Active,
    /// Event administration also covers removed attachments and their retained history.
    IncludingRemoved,
}

/// The locked scope of one reservation writer: the event, its attachments and the accounts, locked
/// in the one lock order, with the actor's rechecked authority.
#[derive(Debug)]
pub struct ReservationScope {
    /// Loaded after the parent lock wait, so time-dependent status reflects the waited instant.
    pub event: Event,
    /// Operational attachments in UUID order.
    pub active_missions: Vec<EventMissionId>,
    /// Every locked attachment, removed ones included when requested, in UUID order.
    pub locked_missions: Vec<EventMissionId>,
    /// The sorted, deduplicated account union locked by this transaction.
    pub accounts: Vec<DiscordUserId>,
    /// Current authority of the actor, rechecked after every lock wait.
    pub actor: Option<AuthUser>,
}

impl ReservationScope {
    /// Lock the event, its attachments in UUID order and the sorted account union (the event's
    /// participants, `named_accounts` and the actor), then recheck the actor's authority. Not
    /// found when the event is deleted; a bad request when a named account names no user.
    pub async fn lock(
        connection: &mut PgConnection,
        event_id: EventId,
        attachments: AttachmentScope,
        actor: Option<(&AuthUser, &Config)>,
        named_accounts: &[DiscordUserId],
    ) -> Result<Self, ApiError> {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM events WHERE id = $1 AND deleted_at IS NULL FOR NO KEY UPDATE",
        )
        .bind(event_id)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or_else(|| ApiError::not_found("event not found"))?;
        let include_removed = attachments == AttachmentScope::IncludingRemoved;
        let locked: Vec<(EventMissionId, bool)> = sqlx::query_as(
            "SELECT id, deleted_at IS NULL FROM event_missions
             WHERE event_id = $1 AND ($2 OR deleted_at IS NULL) ORDER BY id FOR NO KEY UPDATE",
        )
        .bind(event_id)
        .bind(include_removed)
        .fetch_all(&mut *connection)
        .await?;
        let locked_missions: Vec<EventMissionId> = locked.iter().map(|(id, _)| *id).collect();
        let active_missions: Vec<EventMissionId> = locked
            .iter()
            .filter_map(|(id, active)| active.then_some(*id))
            .collect();
        let mut accounts: Vec<DiscordUserId> = sqlx::query_scalar(
            "SELECT discord_id FROM event_registrations WHERE event_mission_id = ANY($1)
             UNION SELECT assigned_to FROM orbat_slots
                 WHERE event_mission_id = ANY($1) AND assigned_to IS NOT NULL
             UNION SELECT discord_id FROM event_participant_allocations
                 WHERE event_id = $2 AND released_at IS NULL",
        )
        .bind(&locked_missions)
        .bind(event_id)
        .fetch_all(&mut *connection)
        .await?;
        accounts.extend_from_slice(named_accounts);
        if let Some((user, _)) = actor {
            accounts.push(user.discord_id.clone());
        }
        accounts.sort_unstable();
        accounts.dedup();
        lock_accounts(connection, &accounts).await?;
        let existing: Vec<DiscordUserId> =
            sqlx::query_scalar("SELECT discord_id FROM users WHERE discord_id = ANY($1)")
                .bind(named_accounts)
                .fetch_all(&mut *connection)
                .await?;
        if named_accounts.iter().any(|named| !existing.contains(named)) {
            return Err(ApiError::bad_request("user not found"));
        }
        let actor = match actor {
            Some((user, config)) => {
                Some(authorize_on_connection(connection, config, &user.session_claims).await?)
            }
            None => None,
        };
        let event = load_locked_event(connection, event_id).await?;
        Ok(Self {
            event,
            active_missions,
            locked_missions,
            accounts,
            actor,
        })
    }

    /// The attachment is still operational after the lock wait.
    pub fn require_active_mission(&self, mission: EventMissionId) -> Result<(), ApiError> {
        if self.active_missions.contains(&mission) {
            Ok(())
        } else {
            Err(ApiError::not_found("mission not found in event"))
        }
    }

    /// The reauthorized actor; system transactions have none.
    pub fn actor(&self) -> Result<&AuthUser, ApiError> {
        self.actor
            .as_ref()
            .ok_or_else(|| ApiError::internal("reservation scope has no actor"))
    }
}
