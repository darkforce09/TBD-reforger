//! The Server-Sent Events feed for one server's live status.
//!
//! The server is resolved before the stream opens: a malformed id answers 400, an unknown server
//! 404, and an inactive one 404 for anyone but an administrator. The connection then opens with
//! the current snapshot so the client renders without waiting for the next publish, and relays
//! every frame the realtime hub fans out on `server:{id}`. Those frames are produced by
//! [`crate::services::status_broadcast`], which is the only writer of the
//! topic, so the snapshot and the stream carry the same shape.

use std::convert::Infallible;

use async_stream::stream;
use axum::extract::State;
use axum::http::HeaderName;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::handlers::server_intel::sees_inactive_servers;
use crate::models::server::{ServerStatus, ServerStatusRow};
use crate::services::status_broadcast::SELECT_SERVER_STATUS;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AuthUser;
use api_http_layer::middleware::authorized_event_stream::authorize_event_stream;
use api_state::AppState;

/// `GET /api/v1/servers/:id/status/stream` — SSE live server-status feed. A malformed id answers
/// 400 and an unknown server 404 before any stream opens; an inactive server is not found for
/// anyone but an administrator.
///
/// @route GET /api/v1/servers/:id/status/stream
pub async fn stream_server_status(
    State(state): State<AppState>,
    viewer: AuthUser,
    PathParams(server_id): PathParams<Uuid>,
) -> Result<Response, ApiError> {
    let active: Option<bool> = sqlx::query_scalar("SELECT is_active FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(&state.pool)
        .await?;
    match active {
        None => return Err(ApiError::not_found("server not found")),
        Some(false) if !sees_inactive_servers(&viewer) => {
            return Err(ApiError::not_found("server not found"));
        }
        Some(_) => {}
    }
    let mut rx = state.hub.subscribe(&format!("server:{server_id}"));
    let pool = state.pool.clone();

    let body = stream! {
        // Current snapshot first, so the client renders without delay.
        let snap: Result<Option<ServerStatusRow>, _> = sqlx::query_as(SELECT_SERVER_STATUS)
            .bind(server_id)
            .fetch_optional(&pool)
            .await;
        let snap = snap.map(|row| row.map(ServerStatus::from));
        if let Ok(Some(status)) = snap
            && let Ok(js) = serde_json::to_string(&status) {
            yield Ok::<Event, Infallible>(Event::default().data(js));
        }
        loop {
            match rx.recv().await {
                Ok(bytes) => yield Ok(Event::default().data(String::from_utf8_lossy(&bytes))),
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    };

    Ok((
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        Sse::new(authorize_event_stream(body, state, viewer, "guest")),
    )
        .into_response())
}
