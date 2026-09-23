//! Account-serialized issuance keeps one pending code and preserves terminal code history.
use super::{account_authority::lock_account, session_authorization::authorize_on_connection};
use crate::administration::services::required_audit::append_required_audit;
use crate::core::{
    application_state::AppState, authentication_primitives, error_handling::api_error::ApiError,
    middleware::AuthUser,
};
use chrono::{DateTime, Utc};

pub async fn issue_link_code(
    state: &AppState,
    user: &AuthUser,
) -> Result<(String, DateTime<Utc>), ApiError> {
    let mut tx = state.pool.begin().await?;
    lock_account(&mut tx, &user.discord_id).await?;
    authorize_on_connection(&mut tx, &state.cfg, &user.session_claims).await?;
    sqlx::query("UPDATE identity_link_codes SET cancelled_at = clock_timestamp(), cancellation_reason = 'superseded'
        WHERE discord_id = $1 AND consumed_at IS NULL AND cancelled_at IS NULL")
        .bind(&user.discord_id).execute(&mut *tx).await?;
    for _ in 0..32 {
        let code = authentication_primitives::numeric_code(6);
        let expires: Option<DateTime<Utc>> = sqlx::query_scalar("INSERT INTO identity_link_codes(code, discord_id, expires_at)
            VALUES ($1, $2, clock_timestamp() + interval '10 minutes') ON CONFLICT (code) DO NOTHING RETURNING expires_at")
            .bind(&code).bind(&user.discord_id).fetch_optional(&mut *tx).await?;
        if let Some(expires) = expires {
            append_required_audit(
                &mut tx,
                &user.discord_id,
                "identity.link_code_issued",
                &user.discord_id,
                "Issued a single-use account linking code; superseded earlier pending codes",
            )
            .await?;
            tx.commit().await?;
            return Ok((code, expires));
        }
    }
    Err(ApiError::internal(
        "could not allocate an unused identity code; retry later",
    ))
}
