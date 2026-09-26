//! The Server-Sent Events feed for one server's live status.
//!
//! The connection opens with the current snapshot so the client renders without waiting for the
//! next publish, then relays every frame the realtime hub fans out on `server:{id}`. Those frames
//! are produced by [`crate::server_infrastructure::services::status_broadcast`], which is the only
//! writer of the topic, so the snapshot and the stream carry the same shape.

use std::convert::Infallible;

use async_stream::stream;
use axum::extract::{Path, State};
use axum::http::HeaderName;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::server_infrastructure::handlers::server_intel::sees_inactive_servers;
use crate::server_infrastructure::models::server::{ServerStatus, ServerStatusRow};
use crate::server_infrastructure::services::status_broadcast::SELECT_SERVER_STATUS;

/// `GET /api/v1/servers/:id/status/stream` — SSE live server-status feed; an inactive server is
/// not found for anyone but an administrator.
///
/// @route GET /api/v1/servers/:id/status/stream
pub async fn stream_server_status(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Response {
    if let Ok(server_id) = Uuid::parse_str(&id) {
        let active: Result<Option<bool>, _> =
            sqlx::query_scalar("SELECT is_active FROM servers WHERE id = $1")
                .bind(server_id)
                .fetch_optional(&state.pool)
                .await;
        match active {
            Ok(Some(false)) if !sees_inactive_servers(&_u) => {
                return ApiError::not_found("server not found").into_response();
            }
            Err(error) => return ApiError::from(error).into_response(),
            _ => {}
        }
    }
    let topic = format!("server:{id}");
    let mut rx = state.hub.subscribe(&topic);
    let pool = state.pool.clone();
    let uuid = Uuid::parse_str(&id).ok();

    let body = stream! {
        // Current snapshot first, so the client renders without delay.
        if let Some(sid) = uuid {
            let snap: Result<Option<ServerStatusRow>, _> = sqlx::query_as(SELECT_SERVER_STATUS)
                .bind(sid)
                .fetch_optional(&pool)
                .await;
            let snap = snap.map(|row| row.map(ServerStatus::from));
            if let Ok(Some(status)) = snap
                && let Ok(js) = serde_json::to_string(&status) {
                yield Ok::<Event, Infallible>(Event::default().data(js));
            }
        }
        loop {
            match rx.recv().await {
                Ok(bytes) => yield Ok(Event::default().data(String::from_utf8_lossy(&bytes))),
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    };

    (
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        Sse::new(
            crate::core::middleware::authorized_event_stream::authorize_event_stream(
                body, state, _u, "guest",
            ),
        ),
    )
        .into_response()
}
