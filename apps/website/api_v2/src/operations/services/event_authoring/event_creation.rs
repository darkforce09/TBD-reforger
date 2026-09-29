//! Creating an event: the field rules the `events` table does not enforce, and the insert that
//! writes the row together with its `event.created` audit row.
//!
//! **Role:** the one path that creates an `events` row, and the field validators every event write
//! applies.
//!
//! **Position:** called by `POST /api/v1/events`
//! ([`create_event`](crate::operations::handlers::event_create_update::create_event)) and by the
//! `staging-fixtures seed-load-fixture-events` host tool; `PATCH /api/v1/events/:id`
//! ([`update_event`](crate::operations::handlers::event_create_update::update_event)) applies the
//! same validators to the fields it changes.
//!
//! **Signals & state:** none; [`create_event`] runs on the connection its caller passes, inside the
//! caller's transaction.
//!
//! **Invariants:** a created event has a start time with a four-digit year, stored to the
//! microsecond; a pre-start status (`scheduled`, `open` or `locked`); 0 to 256 places, 0 leaving
//! it uncapped; a name override that is empty or not blank; a banner that is empty or an absolute
//! http(s) URL; and a server and modpack that exist when it names them. Everything else takes the
//! table's defaults: the access policy that admits verified TBD members, and a member reservation
//! pool that is uncapped and open from creation. The row and its `event.created` audit row commit
//! together or not at all. Authority is the caller's to settle before the insert: the route
//! rechecks the administrator session, and the host tool acts for its reserved fixture author.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::error_handling::api_error::ApiError;
use crate::core::text::http_url_guard::is_http_url;
use crate::operations::models::{Event, EventStatus};
use crate::operations::services::event_reservations::event_administration::{
    load_locked_event, normalize_schedule_time,
};
use crate::operations::services::event_status_rules::{is_pre_start, valid_event_status};

/// The most places an event may cap its registrations at; 0 leaves the event uncapped.
pub const MAXIMUM_EVENT_PLACES: i64 = 256;

/// The fields of a new event as its caller received them, before any rule has run.
#[derive(Debug, Clone, Default)]
pub struct EventCreationRequest {
    /// When the operation starts; required.
    pub start_time: Option<DateTime<Utc>>,
    /// The display name; empty falls back to the first mission's title.
    pub name_override: String,
    /// The briefing text.
    pub briefing: String,
    /// The banner image; empty for none.
    pub banner_image_url: String,
    /// The event-wide place cap; 0 leaves the event uncapped.
    pub max_slots: i64,
    /// Whether registration starts locked.
    pub registration_locked: bool,
    /// The status as the caller spelled it; empty means `scheduled`.
    pub status: String,
    /// The game server the event runs on.
    pub server_id: Option<Uuid>,
    /// The modpack the event requires.
    pub modpack_id: Option<Uuid>,
}

/// A new event whose every field has passed the event rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCreation {
    start_time: DateTime<Utc>,
    name_override: String,
    briefing: String,
    banner_image_url: String,
    max_slots: i64,
    registration_locked: bool,
    status: EventStatus,
    server_id: Option<Uuid>,
    modpack_id: Option<Uuid>,
}

impl EventCreation {
    /// Apply the event rules to `request` in the order the route reports them: the start time,
    /// the place count, the schedule precision, the status, the name override, then the banner.
    /// The server and the modpack are checked against the database by [`create_event`].
    pub fn new(request: EventCreationRequest) -> Result<Self, ApiError> {
        let Some(start_time) = request.start_time else {
            return Err(ApiError::bad_request("start_time is required"));
        };
        if !(0..=MAXIMUM_EVENT_PLACES).contains(&request.max_slots) {
            return Err(ApiError::bad_request("max_slots must be between 0 and 256"));
        }
        let start_time = normalize_schedule_time(start_time)?;
        let Some(status) = valid_event_status(&request.status) else {
            return Err(ApiError::bad_request("invalid status"));
        };
        if !is_pre_start(status) {
            return Err(ApiError::bad_request(
                "an event may only be created as scheduled, open or locked",
            ));
        }
        check_name_override(&request.name_override)?;
        let banner_image_url = validated_banner_image_url(&request.banner_image_url)?;
        Ok(Self {
            start_time,
            name_override: request.name_override,
            briefing: request.briefing,
            banner_image_url,
            max_slots: request.max_slots,
            registration_locked: request.registration_locked,
            status,
            server_id: request.server_id,
            modpack_id: request.modpack_id,
        })
    }

    /// The start time the row stores, at microsecond precision.
    pub fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    /// The name override the row stores.
    pub fn name_override(&self) -> &str {
        &self.name_override
    }

    /// The event-wide place cap the row stores.
    pub fn max_slots(&self) -> i64 {
        self.max_slots
    }

    /// The status the event is created in.
    pub fn status(&self) -> EventStatus {
        self.status
    }
}

/// A banner is empty (none, or a clear on `PATCH`) or an absolute http(s) URL, stored trimmed.
pub fn validated_banner_image_url(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_http_url(trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ApiError::bad_request(
        "banner_image_url must be an absolute http:// or https:// URL",
    ))
}

/// A name override is empty (fall back to the mission's title) or holds a visible character.
pub fn check_name_override(n: &str) -> Result<(), ApiError> {
    if !n.is_empty() && n.trim().is_empty() {
        return Err(ApiError::bad_request(
            "name_override must not be blank — send \"\" to clear it and fall back to the \
             mission's title",
        ));
    }
    Ok(())
}

/// `events.server_id` must name a registered server.
pub async fn require_server<'e>(executor: impl PgExecutor<'e>, id: Uuid) -> Result<(), ApiError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM servers WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "server_id does not name a known server",
        ));
    }
    Ok(())
}

/// `events.modpack_id` must name a known modpack.
pub async fn require_event_modpack<'e>(
    executor: impl PgExecutor<'e>,
    id: Uuid,
) -> Result<(), ApiError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM modpacks WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "modpack_id does not name a known modpack",
        ));
    }
    Ok(())
}

/// Insert a validated event attributed to `author` and append its `event.created` audit row on
/// the same connection, so the caller's transaction commits both or neither.
///
/// A named server and modpack are checked first ([`require_server`], [`require_event_modpack`]).
/// The audit append fails when no account holds `author`. Returns the stored row as the event
/// reads project it.
pub async fn create_event(
    connection: &mut PgConnection,
    creation: &EventCreation,
    author: &str,
) -> Result<Event, ApiError> {
    if let Some(server) = creation.server_id {
        require_server(&mut *connection, server).await?;
    }
    if let Some(modpack) = creation.modpack_id {
        require_event_modpack(&mut *connection, modpack).await?;
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, briefing, banner_image_url, status, \
         registration_locked, max_slots, created_by, server_id, modpack_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now(), now()) RETURNING id",
    )
    .bind(&creation.name_override)
    .bind(creation.start_time)
    .bind(&creation.briefing)
    .bind(&creation.banner_image_url)
    .bind(creation.status)
    .bind(creation.registration_locked)
    .bind(creation.max_slots)
    .bind(author)
    .bind(creation.server_id)
    .bind(creation.modpack_id)
    .fetch_one(&mut *connection)
    .await?;
    let event = load_locked_event(&mut *connection, id).await?;
    append_actor_audit(
        connection,
        author,
        "event.created",
        "event",
        &id.to_string(),
        "Event created with TBD-member access",
    )
    .await?;
    Ok(event)
}

#[cfg(test)]
#[path = "tests/event_creation.rs"]
mod tests;
