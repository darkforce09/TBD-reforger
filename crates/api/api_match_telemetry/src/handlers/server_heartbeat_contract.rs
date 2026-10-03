//! The body of a game-runtime heartbeat, decoded and checked before the session fence.
//!
//! **Role:** the wire shape of `POST /api/v1/game-runtime/sessions/:sessionId/heartbeats`.
//! **Position:** decoded by [`super::server_heartbeat`], which fences and writes it.
//! **Signals & state:** none.
//! **Invariants:** unknown keys are refused; every reading is optional and absent keeps the stored
//! value; the telemetry queue block is all-or-nothing with `backlog <= capacity`.
//! @contract game-runtime-session.schema.json#/definitions/RuntimeHeartbeat

use api_identifiers::RuntimeSessionId;
use serde::Deserialize;

use api_foundation::error_handling::api_error::ApiError;
use api_server_infrastructure::services::runtime_sessions::HeartbeatFence;

/// A live-status heartbeat.
///
/// **Every measurement here is `Option` on purpose, and absent means "no new reading" —
/// do not add `#[serde(default)]` back.** This is the one input struct where requiring the
/// fields would be the *wrong* fix. A heartbeat is a periodic push from a game server, and the
/// wire contract has always allowed a sender to report only what it currently knows — the
/// committed integration test posts a heartbeat with no `uptime_seconds`, `ingame_time` or
/// `ingame_weather`, and the low-FPS case omits `max_players` as well. Making those mandatory
/// would break real senders to fix a bug they don't have.
///
/// The bug the optionality closes is that "absent" decoding as an affirmative **zero** is bound
/// straight into the upsert: a heartbeat carrying only `server_id` + `is_online` overwrites a live
/// `player_count=48, server_fps=58.5, max_players=64, uptime_seconds=7200` row with all
/// zeros, appends a permanent `0 / 0.0` row to `server_status_histories` (a time series —
/// that sample can never be corrected), and trips the `server.low_fps` edge trigger into
/// a WARN about an FPS collapse that never happened.
///
/// So the rule is per-field, not blanket: `None` keeps the stored value (`COALESCE` against
/// the existing row), `Some` sets it. `current_match_id` needs three states rather than two
/// — absent keeps, and an explicit `""` clears — because a match really does end and the
/// live row has to stop pointing at it; the empty-string-means-none convention is what the
/// crate's ingest parser `services::ingest_parsing::parse_uuid_opt` implements. A heartbeat
/// with nothing but its session fence carries no reading at all, so it is a 400 rather than a
/// write of nothing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerStatusInput {
    /// Refused when present: the server is the credential's, never the sender's claim.
    pub(super) server_id: Option<serde::de::IgnoredAny>,
    pub(super) generation: Option<i64>,
    pub(super) sequence: Option<i64>,
    pub(super) is_online: Option<bool>,
    pub(super) player_count: Option<i64>,
    pub(super) max_players: Option<i64>,
    pub(super) server_fps: Option<f64>,
    pub(super) uptime_seconds: Option<i64>,
    /// Absent = leave the current match alone; `""` = clear it; a uuid = set it.
    pub(super) current_match_id: Option<String>,
    /// Absent = keep; `""` / whitespace = clear; otherwise set (trimmed). See
    /// [`crate::services::ingest_parsing::coalesce_str`].
    pub(super) ingame_time: Option<String>,
    /// Absent = keep; `""` / whitespace = clear; otherwise set (trimmed). See
    /// [`crate::services::ingest_parsing::coalesce_str`].
    pub(super) ingame_weather: Option<String>,
    /// Absent = keep the stored queue reading; present = all four fields, `backlog <= capacity`.
    pub(super) telemetry_queue: Option<TelemetryQueueInput>,
}

/// The game runtime's outbound telemetry queue as one heartbeat reports it.
/// @contract match-telemetry.schema.json#/definitions/TelemetryQueueReading
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryQueueInput {
    /// Events waiting in the queue.
    pub backlog: i64,
    /// The most events the queue holds.
    pub capacity: i64,
    /// Events dropped since the runtime started, because the queue was full.
    pub dropped_total: i64,
    /// Age of the oldest waiting event, in seconds.
    pub oldest_age_seconds: i64,
}

impl ServerStatusInput {
    /// The session fence every heartbeat must carry.
    pub(super) fn fence(
        &self,
        runtime_session_id: RuntimeSessionId,
    ) -> Result<HeartbeatFence, ApiError> {
        let (Some(generation), Some(sequence)) = (self.generation, self.sequence) else {
            return Err(ApiError::bad_request(
                "generation and sequence are required",
            ));
        };
        Ok(HeartbeatFence {
            runtime_session_id,
            generation,
            sequence,
        })
    }

    /// True when the body says nothing beyond its session fence.
    pub(super) fn is_empty_reading(&self) -> bool {
        self.is_online.is_none()
            && self.player_count.is_none()
            && self.max_players.is_none()
            && self.server_fps.is_none()
            && self.uptime_seconds.is_none()
            && self.current_match_id.is_none()
            && self.ingame_time.is_none()
            && self.ingame_weather.is_none()
            && self.telemetry_queue.is_none()
    }

    /// The queue reading, refused unless every value is non-negative and `backlog <= capacity`.
    pub(super) fn telemetry_queue(&self) -> Result<Option<TelemetryQueueInput>, ApiError> {
        let Some(queue) = self.telemetry_queue else {
            return Ok(None);
        };
        if queue.backlog < 0
            || queue.capacity < 0
            || queue.dropped_total < 0
            || queue.oldest_age_seconds < 0
        {
            return Err(ApiError::bad_request(
                "telemetry_queue values must not be negative",
            ));
        }
        if queue.backlog > queue.capacity {
            return Err(ApiError::bad_request(
                "telemetry_queue backlog must not exceed capacity",
            ));
        }
        Ok(Some(queue))
    }
}
