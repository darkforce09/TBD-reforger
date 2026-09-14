//! Game-server rows, their live telemetry, and the decoder for the telemetry stream.
//!
//! **Role:** the server list the intel page renders, the status frame the live stream pushes,
//! and the audit line written when a frame cannot be read.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** frame decoding lives here rather than beside the stream reader because the reader is
//! browser-only and therefore never compiled by the native test build, while decoding a frame is
//! wire-contract work that must be tested. A frame that fails to deserialise becomes a named
//! rejection, never silence: "the app cannot read this backend's frames" and "no frame has arrived
//! yet" are different facts, and rendering the second while the first is true is the bug the
//! distinction exists to prevent.

use serde::{Deserialize, Serialize};

use super::content::ModpackDto;

/// One live telemetry sample from a game server.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerStatusDto {
    pub server_id: String,
    pub is_online: bool,
    pub player_count: i64,
    pub max_players: i64,
    /// Stored as a one-decimal numeric on the backend, so it crosses the wire as a float.
    pub server_fps: f64,
    pub uptime_seconds: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_match_id: Option<String>,
    /// The backend omits this rather than sending an empty string, so absent is the only
    /// encoding of "no value" and an option round-trips it exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingame_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingame_weather: Option<String>,
    pub updated_at: String,
}

/// What one frame from the telemetry stream turned out to be.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum SseFrame {
    /// A data frame that deserialised into a status. Boxed, or the variant would make the enum
    /// as large as the whole payload.
    Status(Box<ServerStatusDto>),
    /// A data frame that did not deserialise, carrying the parse error and the payload that
    /// caused it.
    Rejected { error: String, payload: String },
    /// Not a data frame — a keepalive comment, a stream control line, or the empty tail.
    /// Silence is correct here; auditing these would drown the signal that matters.
    NotData,
}

/// Decode one raw frame — the text between blank-line boundaries — into its kind.
#[allow(dead_code)]
pub fn decode_server_status_frame(frame: &str) -> SseFrame {
    let Some(data) = frame.trim().strip_prefix("data:") else {
        return SseFrame::NotData;
    };
    let data = data.trim();
    match serde_json::from_str::<ServerStatusDto>(data) {
        Ok(dto) => SseFrame::Status(Box::new(dto)),
        Err(e) => SseFrame::Rejected {
            error: e.to_string(),
            payload: data.chars().take(PAYLOAD_AUDIT_CHARS).collect(),
        },
    }
}

/// How many characters of a rejected payload the audit line quotes. Enough to identify
/// the frame, short enough not to flood the console.
pub const PAYLOAD_AUDIT_CHARS: usize = 400;

/// Build the console line written when a frame is rejected, quoting a bounded prefix of
/// the payload.
pub fn audit_rejected_frame(context: &str, error: &str, payload: &str) -> String {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static SEEN: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    }
    let n = SEEN.with(|s| {
        let mut s = s.borrow_mut();
        let c = s.entry(error.to_string()).or_insert(0);
        *c += 1;
        *c
    });
    let msg = format!(
        "[t306] {context}: telemetry frame REJECTED and dropped — {error}. The stream is connected \
         and the payload arrived intact, so this is a DTO/wire contract mismatch, not a network \
         fault: dto.rs ServerStatusDto disagrees with api/src/models/telemetry.rs ServerStatus. \
         {n} dropped so far with this error. Payload: {payload}"
    );
    if n == 1 || is_power_of_ten(n) {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&msg));
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("{msg}");
    }
    msg
}

/// Whether `n` is a power of ten, used to thin repeated audit lines to a logarithmic
/// sample instead of one line per frame.
pub(crate) fn is_power_of_ten(n: u64) -> bool {
    let mut p = 10u64;
    loop {
        if p == n {
            return true;
        }
        if p > n {
            return false;
        }
        match p.checked_mul(10) {
            Some(next) => p = next,
            None => return false,
        }
    }
}

/// One game server as the intel page lists it.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerRowDto {
    pub id: String,
    pub name: String,
    /// Stored as a network address on the backend and served as text.
    pub ip: String,
    pub port: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_modpack_id: Option<String>,
    pub is_active: bool,
    pub status: Option<ServerStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_modpack: Option<ModpackDto>,
    /// The theatre the current match runs on, and null when the server is between matches.
    pub terrain: Option<String>,
}
