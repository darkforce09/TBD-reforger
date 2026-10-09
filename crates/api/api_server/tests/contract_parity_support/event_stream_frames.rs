//! Server-sent event frames: splitting a stream's bytes and reading a frame's fields.
//!
//! **Role:** turns the leading bytes of an event stream, live or golden, into its complete
//! frames and exposes each frame's `event:` name and `data:` JSON.
//!
//! **Position:** used by the capture in [`super::seeded_capture`] (how many frames to read), by
//! the event-stream case of `tests/contract_parity_goldens.rs` (frame-by-frame comparison) and by
//! [`super::route_contracts`] (frame validation).
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** a frame is the text before a blank line (`\n\n`); a trailing partial frame is
//! never returned, so a stream cut mid-frame shows as a missing frame rather than a short one.

use serde_json::Value;

/// The complete frames at the start of `bytes`, each without its terminating blank line.
pub(crate) fn complete_frames(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut frames: Vec<String> = text.split("\n\n").map(str::to_string).collect();
    // The piece after the last separator is either empty (the stream ended on a frame boundary)
    // or a frame still arriving; neither is a complete frame.
    frames.pop();
    frames
}

/// The frame's `event:` name, or `""` for an unnamed frame.
pub(crate) fn frame_event(frame: &str) -> &str {
    frame
        .lines()
        .find_map(|line| line.strip_prefix("event:"))
        .map(|name| name.strip_prefix(' ').unwrap_or(name))
        .unwrap_or("")
}

/// The frame's `data:` lines joined by newlines, parsed as JSON.
///
/// # Errors
/// Says so when the frame has no data line or the data is not JSON.
pub(crate) fn frame_json(frame: &str) -> Result<Value, String> {
    let lines: Vec<&str> = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|data| data.strip_prefix(' ').unwrap_or(data))
        .collect();
    if lines.is_empty() {
        return Err("the frame carries no data line".to_string());
    }
    serde_json::from_str(&lines.join("\n"))
        .map_err(|error| format!("the data is not JSON: {error}"))
}
