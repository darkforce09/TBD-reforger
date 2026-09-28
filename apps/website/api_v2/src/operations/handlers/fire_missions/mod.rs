//! The fire-mission routes: the save of a re-solved fire mission and an event's saved list.
//!
//! **Role:** HTTP handlers of `POST /api/v1/fire-missions` ([`save`]) and
//! `GET /api/v1/events/{id}/fire-missions` ([`list`]), and the event access check both share.
//!
//! **Position:** registered by [`crate::operations::routes::routes`]; the save re-solves through
//! [`crate::operations::services::fire_mission_resolve`] and both routes read and write through
//! [`crate::operations::services::fire_mission_store`]. The event check goes through
//! [`crate::operations::services::event_access::visibility::viewer_event_access`].
//!
//! **Signals & state:** none.
//!
//! **Invariants:**
//! - Both routes take `AuthUser`.
//! - An event that does not exist, is deleted, or is hidden from the caller answers the same
//!   `404 event not found`; an event the caller sees only partially (through squad or slot
//!   policies) answers 403, because its fire missions belong to the whole event.

pub mod list;
pub mod save;

use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::operations::services::event_access::visibility::{EventVisibility, viewer_event_access};

/// Admits `user` to the fire missions of `event`: full visibility of the event, else the 404
/// or 403 of the module invariants.
async fn require_full_event_access(
    state: &AppState,
    user: &AuthUser,
    event: Uuid,
) -> Result<(), ApiError> {
    let mut connection = state.pool.acquire().await?;
    let access = viewer_event_access(
        &mut connection,
        &user.discord_id,
        user.role == "admin",
        &[event],
        &state.cfg.discord_guild_id,
    )
    .await?
    .remove(&event);
    match access.map(|access| access.visibility) {
        Some(EventVisibility::Full) => Ok(()),
        Some(EventVisibility::Partial { .. }) => Err(ApiError::forbidden(
            "fire missions need full access to the event",
        )),
        Some(EventVisibility::Hidden) | None => Err(ApiError::not_found("event not found")),
    }
}
