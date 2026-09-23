//! The lock every author-or-admin mission write takes: the live mission row, then the actor's
//! account, then the actor's authority re-read after the lock waits. Mission catalog locks come
//! before account locks, so attachment and authority changes serialize without event locks.

use sqlx::PgConnection;

use crate::core::configuration::Config;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AuthUser, role_rank};
use crate::identity_and_access::services::{
    identity_ownership::lock_accounts, session_authorization::authorize_on_connection,
};
use crate::missions::models::mission::{Mission, MissionStatus};
use crate::missions::validation::access::can_edit;

/// Lock `mission` `FOR NO KEY UPDATE`, refresh its status and author from the locked row, and
/// confirm that `actor` is still a mission maker who may edit it. A deleted mission answers 404;
/// a demoted actor or one who no longer owns the mission answers 403.
pub async fn lock_editable_mission(
    connection: &mut PgConnection,
    mission: &mut Mission,
    actor: &AuthUser,
    config: &Config,
) -> Result<(), ApiError> {
    let current: Option<(MissionStatus, String)> = sqlx::query_as(
        "SELECT status, author_id FROM missions WHERE id = $1 AND deleted_at IS NULL FOR NO KEY UPDATE",
    )
    .bind(mission.id)
    .fetch_optional(&mut *connection)
    .await?;
    let (status, author_id) = current.ok_or_else(|| ApiError::not_found("mission not found"))?;
    mission.status = status;
    mission.author_id = author_id;
    lock_accounts(connection, std::slice::from_ref(&actor.discord_id)).await?;
    let current = authorize_on_connection(connection, config, &actor.session_claims).await?;
    if role_rank(&current.role) < role_rank("mission_maker") {
        return Err(ApiError::forbidden("insufficient role"));
    }
    if !can_edit(&current, mission) {
        return Err(ApiError::forbidden("not your mission"));
    }
    Ok(())
}
