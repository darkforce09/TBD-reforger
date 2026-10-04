//! The live audit stream: one authenticated, resumable connection per audit page.
//!
//! **Role:** the protocol of `GET /api/v1/admin/audit-logs/stream`: what each event parsed by
//! [`super::sse_frames`] means ([`AuditStreamStep`]), the resume cursor, the reconnect wait, and
//! what a connection attempt's status means; plus the browser transport that applies them.
//! **Position:** called by the audit logs page, which owns the returned `AuditStreamHandle`. This
//! file is the pure half (natively, the tests are its only caller); the transport in
//! `audit_stream/transport.rs` compiles for `wasm32` only and drives it.
//! **Signals & state:** the tracker holds the resume cursor (a publication sequence, never an
//! audit line id), the failure count and the per-connection flags; the handle shares a stop flag
//! and the live abort controller with the transport task, which only calls the page's callbacks.
//! **Invariants:** the transport is a bearer-authenticated fetch over a readable stream, because
//! the browser's event source cannot carry an authorization header. Every reconnect sends the
//! cursor as `Last-Event-ID`. `ready` on a connection opened without a cursor means the history
//! must be (re)loaded after it; `reset` always means it. The wait after a drop doubles from 1 s
//! (or the server's longer `retry`) to a 30 s ceiling, plus up to a quarter of itself as jitter;
//! anything after `ready` resets it. A `401` revalidates the session once and retries at once; a
//! second `401` or a `403` stops the stream for good, and so does an abort, after which no
//! callback runs.

use super::sse_frames::{DEFAULT_EVENT_TYPE, SseItem, SseMessage};
use frontend_api_dtos::administration::{AuditLogEntry, AuditStreamReady, AuditStreamReset};
use frontend_api_dtos::identifiers::ServerSentEventId;
use serde::de::DeserializeOwned;

/// The stream's path, relative to the API root.
pub const AUDIT_STREAM_PATH: &str = "/admin/audit-logs/stream";

/// The first wait after a drop, in milliseconds.
pub const FIRST_RECONNECT_DELAY_MS: u32 = 1_000;

/// The ceiling the doubling wait stops at, in milliseconds, before jitter.
pub const MAX_RECONNECT_DELAY_MS: u32 = 30_000;

/// What one parsed stream item means to the page.
#[derive(Clone, Debug, PartialEq)]
pub enum AuditStreamStep {
    /// `event: ready`: the stream is open at its cursor. `history_required` is true when the
    /// connection opened without a cursor, so the page must (re)load its history after this event.
    Ready {
        /// The `ready` event's data: the cursor the stream opened at.
        ready: AuditStreamReady,
        /// Whether the page must (re)load its history after this event.
        history_required: bool,
    },
    /// One published audit line.
    Row(Box<AuditLogEntry>),
    /// `event: reset`: the cursor cannot be replayed; the page reloads its history.
    Reset(AuditStreamReset),
    /// `event: authorization_expired`: the server ends the stream; the session is revalidated
    /// before the next connection.
    AuthorizationExpired,
    /// An event whose data does not decode: its type and the decoder's error.
    Rejected {
        /// The event's type.
        event: String,
        /// The decoder's error.
        error: String,
    },
    /// A keep-alive, a retry hint, or an event the page does not use.
    Nothing,
}

/// Why the stream stopped for good.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfflineReason {
    /// The session is gone or could not be revalidated.
    SignedOut,
    /// The server refused the viewer (`403`): the account no longer holds the admin role.
    Forbidden,
}

/// The connection state the page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditStreamState {
    /// The first connection attempt is in flight.
    Connecting,
    /// `ready` arrived and the connection is open.
    Live,
    /// The connection dropped; the next attempt is waiting out its backoff or in flight.
    Reconnecting,
    /// The stream stopped for good.
    Offline(OfflineReason),
}

/// What the transport does with the status a connection attempt answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectVerdict {
    /// A 2xx answer: read the stream.
    Stream,
    /// A `401` with no revalidation since the last open: revalidate, then retry at once.
    Revalidate,
    /// Stop for good.
    Stop(OfflineReason),
    /// A `400`: the cursor was refused. Forget it (the next `ready` reloads history), back off.
    ForgetCursorAndBackOff,
    /// Anything else, a network failure (status `0`) included: back off and retry.
    BackOff,
}

/// Decide what a connection attempt's status means. `revalidated` is whether the session was
/// already revalidated since the last connection that opened.
pub fn connect_verdict(status: u16, revalidated: bool) -> ConnectVerdict {
    match status {
        200..=299 => ConnectVerdict::Stream,
        401 if revalidated => ConnectVerdict::Stop(OfflineReason::SignedOut),
        401 => ConnectVerdict::Revalidate,
        403 => ConnectVerdict::Stop(OfflineReason::Forbidden),
        400 => ConnectVerdict::ForgetCursorAndBackOff,
        _ => ConnectVerdict::BackOff,
    }
}

/// A cursor read from an event id: a non-negative publication sequence, or none.
pub fn parse_cursor(last_event_id: &ServerSentEventId) -> Option<i64> {
    last_event_id
        .as_str()
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|v| *v >= 0)
}

/// The resume and backoff bookkeeping of one audit stream, across its connections.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditStreamTracker {
    /// The last event id received, sent as `Last-Event-ID` on the next connection.
    cursor: Option<i64>,
    /// Drops since the last healthy connection.
    failures: u32,
    /// The server's `retry` hint, in milliseconds, when it sent one.
    server_retry_ms: Option<u64>,
    /// The current connection was opened with a cursor.
    opened_with_cursor: bool,
    /// The current connection delivered `ready`.
    ready_seen: bool,
}

impl AuditStreamTracker {
    /// A tracker for a stream that has received nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a connection, returning the `Last-Event-ID` value to send, if any.
    pub fn begin_connection(&mut self) -> Option<String> {
        self.opened_with_cursor = self.cursor.is_some();
        self.ready_seen = false;
        self.cursor.map(|c| c.to_string())
    }

    /// Forget the cursor, so the next connection starts at the tail and reloads the history.
    pub fn forget_cursor(&mut self) {
        self.cursor = None;
    }

    /// Account for one parsed item of the current connection and say what it means.
    pub fn observe(&mut self, item: SseItem) -> AuditStreamStep {
        if self.ready_seen {
            // Anything after `ready` proves the connection is really held open.
            self.failures = 0;
        }
        let message = match item {
            SseItem::Message(message) => message,
            SseItem::Retry(ms) => {
                self.server_retry_ms = Some(ms);
                return AuditStreamStep::Nothing;
            }
            SseItem::Comment => return AuditStreamStep::Nothing,
        };
        let id = parse_cursor(&message.last_event_id);
        match message.event.as_str() {
            "ready" => match decode::<AuditStreamReady>(&message) {
                Ok(ready) => {
                    self.cursor = id.or(Some(ready.resume_after));
                    self.ready_seen = true;
                    AuditStreamStep::Ready {
                        ready,
                        history_required: !self.opened_with_cursor,
                    }
                }
                Err(error) => rejected(&message, &error),
            },
            "reset" => match decode::<AuditStreamReset>(&message) {
                Ok(reset) => {
                    self.cursor = id.or(Some(reset.resume_after));
                    AuditStreamStep::Reset(reset)
                }
                Err(error) => rejected(&message, &error),
            },
            DEFAULT_EVENT_TYPE => {
                // The id moves past an undecodable row too: replaying it would fail the same way.
                self.cursor = id;
                match decode::<AuditLogEntry>(&message) {
                    Ok(entry) => AuditStreamStep::Row(Box::new(entry)),
                    Err(error) => rejected(&message, &error),
                }
            }
            "authorization_expired" => AuditStreamStep::AuthorizationExpired,
            _ => AuditStreamStep::Nothing,
        }
    }

    /// The current connection ended or failed: the wait before the next attempt, in
    /// milliseconds. `jitter` is a uniform draw from `[0, 1)`.
    pub fn next_delay_ms(&mut self, jitter: f64) -> u32 {
        let first = self.server_retry_ms.map_or(FIRST_RECONNECT_DELAY_MS, |ms| {
            u32::try_from(ms).unwrap_or(u32::MAX)
        });
        let delay = reconnect_delay_ms(first, self.failures, jitter);
        self.failures = self.failures.saturating_add(1);
        delay
    }
}

/// The wait before reconnect attempt `failures` (0 for the first): the nominal delay doubles from
/// `first_ms`, held between [`FIRST_RECONNECT_DELAY_MS`] and [`MAX_RECONNECT_DELAY_MS`], up to
/// [`MAX_RECONNECT_DELAY_MS`], and `jitter` in `[0, 1)` adds up to a quarter of it so that pages
/// dropped together do not return together.
pub fn reconnect_delay_ms(first_ms: u32, failures: u32, jitter: f64) -> u32 {
    let first = first_ms.clamp(FIRST_RECONNECT_DELAY_MS, MAX_RECONNECT_DELAY_MS);
    let doubled = first.saturating_mul(1u32 << failures.min(5));
    let nominal = doubled.min(MAX_RECONNECT_DELAY_MS);
    let spread = f64::from(nominal / 4) * jitter.clamp(0.0, 1.0);
    nominal + spread as u32
}

/// Decode an event's data.
fn decode<T: DeserializeOwned>(message: &SseMessage) -> Result<T, serde_json::Error> {
    serde_json::from_str(&message.data)
}

/// The step for an event whose data did not decode, naming its type and the reason.
fn rejected(message: &SseMessage, error: &serde_json::Error) -> AuditStreamStep {
    AuditStreamStep::Rejected {
        event: message.event.clone(),
        error: error.to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
mod transport;

#[cfg(target_arch = "wasm32")]
pub use transport::{AuditStreamCallbacks, AuditStreamHandle, open_audit_stream};

#[cfg(test)]
#[path = "tests/audit_stream.rs"]
mod tests;
