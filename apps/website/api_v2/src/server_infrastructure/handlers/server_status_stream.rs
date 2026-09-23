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
use crate::core::middleware::AuthUser;
use crate::server_infrastructure::models::server::ServerStatus;

/// `GET /api/v1/servers/:id/status/stream` — SSE live server-status feed.
///
/// @route GET /api/v1/servers/:id/status/stream
pub async fn stream_server_status(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Response {
    let topic = format!("server:{id}");
    let mut rx = state.hub.subscribe(&topic);
    let pool = state.pool.clone();
    let uuid = Uuid::parse_str(&id).ok();

    let body = stream! {
        // Current snapshot first, so the client renders without delay.
        if let Some(sid) = uuid {
            let snap: Result<Option<ServerStatus>, _> = sqlx::query_as(
                "SELECT server_id, is_online, player_count, max_players, server_fps::float8 AS server_fps, uptime_seconds, current_match_id, COALESCE(ingame_time, '') AS ingame_time, COALESCE(ingame_weather, '') AS ingame_weather, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM server_statuses WHERE server_id = $1",
            ).bind(sid).fetch_optional(&pool).await;
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
