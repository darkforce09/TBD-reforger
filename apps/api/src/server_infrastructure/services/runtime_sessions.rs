//! Runtime sessions: one boot of a server's game runtime, fenced by a per-server generation.
//!
//! Starting a session ends the server's open one as `superseded` and takes the next generation.
//! A heartbeat names its session, generation and a sequence that must strictly increase within
//! the session, so a delayed, duplicated or stale-runtime message is refused instead of
//! overwriting newer live state. A session with no heartbeat for [`SESSION_EXPIRY_SECONDS`]
//! expires. Ending a session ends the player lives still open in it. Lock order: server, then
//! runtime session rows, then their live occupancies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::administration::services::required_audit::append_system_audit;
use crate::core::error_handling::api_error::ApiError;
use crate::core::wire_format::rfc3339_utc;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

/// How often a runtime reports; stated to the runtime when its session starts.
pub const HEARTBEAT_INTERVAL_SECONDS: i64 = 15;
/// A session silent for this long has ended; four missed heartbeats.
pub const SESSION_EXPIRY_SECONDS: i64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEndReason {
    Superseded,
    Expired,
    EndedByRuntime,
    CredentialRevoked,
}

impl SessionEndReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Superseded => "superseded",
            Self::Expired => "expired",
            Self::EndedByRuntime => "ended_by_runtime",
            Self::CredentialRevoked => "credential_revoked",
        }
    }

    /// Why the session's still-open player lives ended.
    fn life_end_reason(self) -> &'static str {
        match self {
            Self::Superseded => "session_superseded",
            Self::Expired => "session_expired",
            Self::EndedByRuntime => "session_ended",
            Self::CredentialRevoked => "credential_revoked",
        }
    }
}

/// `POST /game-runtime/sessions` response.
#[derive(Debug, Serialize)]
pub struct StartedRuntimeSession {
    pub runtime_session_id: Uuid,
    pub server_id: Uuid,
    pub generation: i64,
    #[serde(with = "rfc3339_utc")]
    pub started_at: DateTime<Utc>,
    pub heartbeat_interval_seconds: i64,
    pub expires_after_seconds: i64,
}

/// The fencing token every heartbeat carries.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatFence {
    pub runtime_session_id: Uuid,
    pub generation: i64,
    pub sequence: i64,
}

fn fence_refusal(code: &str, message: String, details: serde_json::Value) -> ApiError {
    let mut details = details;
    details["code"] = serde_json::Value::from(code);
    ApiError::with_details(axum::http::StatusCode::CONFLICT, message, details)
}

/// End one open session and every player life still open in it: a life cannot outlast the
/// runtime that hosts it. The caller holds the session row lock.
async fn end_session(
    connection: &mut PgConnection,
    session: Uuid,
    reason: SessionEndReason,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE server_runtime_sessions SET ended_at = clock_timestamp(), end_reason = $2
         WHERE id = $1 AND ended_at IS NULL",
    )
    .bind(session)
    .bind(reason.as_str())
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE live_slot_occupancies SET ended_at = clock_timestamp(), end_reason = $2
         WHERE runtime_session_id = $1 AND ended_at IS NULL",
    )
    .bind(session)
    .bind(reason.life_end_reason())
    .execute(connection)
    .await?;
    Ok(())
}

/// `POST /game-runtime/sessions` body: the artifact the runtime loaded, when it runs one. Both
/// fields come together; the SHA-256 is of the exact document bytes the runtime loaded.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadedArtifactReport {
    pub loaded_artifact_id: Option<Uuid>,
    pub loaded_artifact_sha256: Option<String>,
}

impl LoadedArtifactReport {
    /// The reported artifact and digest, or `None` when the runtime runs no artifact.
    fn validated(&self) -> Result<Option<(Uuid, &str)>, ApiError> {
        match (
            self.loaded_artifact_id,
            self.loaded_artifact_sha256.as_deref(),
        ) {
            (None, None) => Ok(None),
            (Some(artifact), Some(sha256))
                if sha256.len() == 64
                    && sha256
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
            {
                Ok(Some((artifact, sha256)))
            }
            _ => Err(ApiError::bad_request(
                "loaded_artifact_id and loaded_artifact_sha256 (64 lowercase hex) come together",
            )),
        }
    }
}

/// Start the next generation for the caller's server, superseding its open session, and record
/// the artifact the runtime reported loading.
pub async fn start_runtime_session(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    report: &LoadedArtifactReport,
) -> Result<StartedRuntimeSession, ApiError> {
    let loaded = report.validated()?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM servers WHERE id = $1 AND is_active FOR NO KEY UPDATE",
    )
    .bind(caller.server_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::forbidden("the credential's server is deactivated"))?;
    let open: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM server_runtime_sessions WHERE server_id = $1 AND ended_at IS NULL FOR NO KEY UPDATE",
    )
    .bind(caller.server_id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some((artifact, _)) = loaded {
        let known: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM mission_artifacts WHERE id = $1)")
                .bind(artifact)
                .fetch_one(&mut *connection)
                .await?;
        if !known {
            return Err(ApiError::with_details(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                "the runtime reports an artifact the platform does not know",
                serde_json::json!({ "code": "UNKNOWN_ARTIFACT", "artifact_id": artifact }),
            ));
        }
    }
    if let Some(previous) = open {
        end_session(connection, previous, SessionEndReason::Superseded).await?;
    }
    let (runtime_session_id, generation, started_at): (Uuid, i64, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO server_runtime_sessions (server_id, credential_id, generation,
             loaded_artifact_id, loaded_artifact_sha256)
         SELECT $1, $2, COALESCE(max(generation), 0) + 1, $3, $4
         FROM server_runtime_sessions WHERE server_id = $1
         RETURNING id, generation, started_at",
    )
    .bind(caller.server_id)
    .bind(caller.credential_id)
    .bind(loaded.map(|(artifact, _)| artifact))
    .bind(loaded.map(|(_, sha256)| sha256))
    .fetch_one(&mut *connection)
    .await?;
    append_system_audit(
        connection,
        "server.runtime_session_started",
        "server",
        &caller.server_id.to_string(),
        &format!(
            "Runtime session generation {generation} started{}{}",
            loaded.map_or(String::new(), |(artifact, sha256)| format!(
                " running artifact {artifact} ({sha256})"
            )),
            open.map_or(String::new(), |previous| format!(
                "; superseded session {previous}"
            ))
        ),
    )
    .await?;
    Ok(StartedRuntimeSession {
        runtime_session_id,
        server_id: caller.server_id,
        generation,
        started_at,
        heartbeat_interval_seconds: HEARTBEAT_INTERVAL_SECONDS,
        expires_after_seconds: SESSION_EXPIRY_SECONDS,
    })
}

#[derive(sqlx::FromRow)]
struct SessionState {
    server_id: Uuid,
    generation: i64,
    last_sequence: i64,
    end_reason: Option<String>,
}

async fn lock_session(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    session: Uuid,
) -> Result<SessionState, ApiError> {
    let state: SessionState = sqlx::query_as(
        "SELECT server_id, generation, last_sequence, end_reason FROM server_runtime_sessions
         WHERE id = $1 FOR NO KEY UPDATE",
    )
    .bind(session)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("runtime session not found"))?;
    caller.require_server(state.server_id)?;
    Ok(state)
}

/// Hold the caller's open session for the rest of the transaction: a session cannot end while
/// a decision that depends on it commits. Refuses another server's or an ended session.
pub async fn share_open_session(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    session: Uuid,
) -> Result<(), ApiError> {
    let (server_id, end_reason): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT server_id, end_reason FROM server_runtime_sessions WHERE id = $1 FOR SHARE",
    )
    .bind(session)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("runtime session not found"))?;
    caller.require_server(server_id)?;
    match end_reason {
        None => Ok(()),
        Some(reason) => Err(fence_refusal(
            "RUNTIME_SESSION_ENDED",
            format!("runtime session has ended ({reason})"),
            serde_json::json!({ "end_reason": reason }),
        )),
    }
}

/// End the caller's own session; ending an already ended session reports how it ended.
pub async fn end_runtime_session(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    session: Uuid,
) -> Result<String, ApiError> {
    let state = lock_session(connection, caller, session).await?;
    if let Some(reason) = state.end_reason {
        return Ok(reason);
    }
    end_session(connection, session, SessionEndReason::EndedByRuntime).await?;
    Ok(SessionEndReason::EndedByRuntime.as_str().to_owned())
}

/// Admit a heartbeat for the caller's open session and advance its sequence, or refuse it.
pub async fn admit_heartbeat(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    fence: HeartbeatFence,
) -> Result<(), ApiError> {
    if fence.generation < 1 || fence.sequence < 1 {
        return Err(ApiError::bad_request(
            "generation and sequence must be positive",
        ));
    }
    let state = lock_session(connection, caller, fence.runtime_session_id).await?;
    if state.generation != fence.generation {
        return Err(fence_refusal(
            "STALE_GENERATION",
            format!("runtime session is generation {}", state.generation),
            serde_json::json!({ "generation": state.generation }),
        ));
    }
    if let Some(reason) = state.end_reason {
        return Err(fence_refusal(
            "RUNTIME_SESSION_ENDED",
            format!("runtime session has ended ({reason})"),
            serde_json::json!({ "end_reason": reason }),
        ));
    }
    if fence.sequence <= state.last_sequence {
        return Err(fence_refusal(
            "STALE_SEQUENCE",
            format!("heartbeat sequence must exceed {}", state.last_sequence),
            serde_json::json!({ "last_sequence": state.last_sequence }),
        ));
    }
    sqlx::query(
        "UPDATE server_runtime_sessions SET last_sequence = $2, last_heartbeat_at = clock_timestamp()
         WHERE id = $1",
    )
    .bind(fence.runtime_session_id)
    .bind(fence.sequence)
    .execute(connection)
    .await?;
    Ok(())
}

/// End every open session a credential authenticated. The caller holds the credential lock.
pub async fn end_sessions_of_credential(
    connection: &mut PgConnection,
    credential: Uuid,
    reason: SessionEndReason,
) -> Result<u64, ApiError> {
    let open: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM server_runtime_sessions WHERE credential_id = $1 AND ended_at IS NULL
         ORDER BY id FOR NO KEY UPDATE",
    )
    .bind(credential)
    .fetch_all(&mut *connection)
    .await?;
    for session in &open {
        end_session(connection, *session, reason).await?;
    }
    Ok(open.len() as u64)
}

/// End sessions silent for [`SESSION_EXPIRY_SECONDS`] and mark their servers offline.
/// Returns the servers whose status changed. Sessions another transaction holds are skipped
/// and examined on the next pass.
pub async fn expire_silent_runtime_sessions(pool: &PgPool) -> Result<Vec<Uuid>, ApiError> {
    let mut transaction = pool.begin().await?;
    let silent: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, server_id FROM server_runtime_sessions
         WHERE ended_at IS NULL
           AND COALESCE(last_heartbeat_at, started_at) < clock_timestamp() - make_interval(secs => $1)
         ORDER BY id FOR NO KEY UPDATE SKIP LOCKED",
    )
    .bind(SESSION_EXPIRY_SECONDS as f64)
    .fetch_all(&mut *transaction)
    .await?;
    let mut servers = Vec::with_capacity(silent.len());
    for (session, server) in silent {
        end_session(&mut transaction, session, SessionEndReason::Expired).await?;
        sqlx::query("UPDATE server_statuses SET is_online = false, updated_at = clock_timestamp() WHERE server_id = $1")
            .bind(server)
            .execute(&mut *transaction)
            .await?;
        append_system_audit(
            &mut transaction,
            "server.runtime_session_expired",
            "server",
            &server.to_string(),
            &format!(
                "Runtime session {session} sent no heartbeat for {SESSION_EXPIRY_SECONDS} seconds"
            ),
        )
        .await?;
        servers.push(server);
    }
    transaction.commit().await?;
    Ok(servers)
}
