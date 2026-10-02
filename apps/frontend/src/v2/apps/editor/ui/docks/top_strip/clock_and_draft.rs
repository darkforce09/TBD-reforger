//! Clock and draft for the top command strip.

use super::*;

/// Format minutes since midnight as an HH:MM clock.
pub fn minutes_to_hhmm(min: u32) -> String {
    format!("{:02}:{:02}", (min / 60) % 24, min % 60)
}

/// Parse HH:MM or HH:MM:SS into minutes since midnight, rejecting invalid
/// clock fields. Database text hydration can include seconds.
pub fn hhmm_to_minutes(s: &str) -> Option<u32> {
    let mut parts = s.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    if let Some(sec) = parts.next() {
        let sec: u32 = sec.parse().ok()?;
        if sec > 59 {
            return None;
        }
    }
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// A clock from any of the controls (or from the row hydrate) → canonical `HH:MM`; `None` when it
/// is not one. The shape [`RowMirror::set_time`] sends to `PATCH /missions/{id}`.
pub fn normalize_clock(s: &str) -> Option<String> {
    hhmm_to_minutes(s).map(minutes_to_hhmm)
}

/// Format a draft save timestamp as a coarse recency phrase. The one-second
/// refresh tick limits the displayed precision.
#[must_use]
pub fn format_draft_recency(elapsed_ms: f64) -> String {
    format!("Draft saved {}", draft_recency_phrase(elapsed_ms))
}

/// Under this gap the chip says "just now" rather than "0s ago" / "1s ago". Also the floor that
/// absorbs backwards clock skew (a resync between the flush stamp and the render tick).
pub(super) const RECENCY_JUST_NOW_MS: f64 = 5_000.0;

/// The recency phrase alone (no "Draft saved " prefix) — split out so the pin can assert the ladder
/// without threading the prefix through every case.
pub(super) fn draft_recency_phrase(elapsed_ms: f64) -> String {
    if elapsed_ms.is_nan() || elapsed_ms < RECENCY_JUST_NOW_MS {
        return "just now".to_string();
    }
    let secs = (elapsed_ms / 1_000.0) as u64;
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3_600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3_600)
    }
}

/// wording). `local draft` (this, automatic, in this browser) versus `Save Version` (the durable
/// library entry). Author-facing, so it names neither IndexedDB nor the debounce — it answers the
/// only question the chip raises: "is this the same as saving?"
pub(super) const DRAFT_CHIP_TOOLTIP: &str =
    "Your work is auto-saved as a local draft in this browser, so \
     closing the tab will not cost you the session — use Save Version to publish a durable version \
     to the mission library.";

///
/// The editor also mounts on synthetic ids — `mission_editor` falls back to `draft`, and the gate
/// route drives a smoke id — where a row PATCH is a guaranteed 400. Cheap shape check rather than a
/// `uuid` dependency the SPA does not otherwise carry.
///
/// guards. Keeping a second copy as `is_row_id` invited drift; one predicate, one owner.
pub(crate) fn is_mission_row_id(s: &str) -> bool {
    s.len() == 36
        && s.as_bytes().iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}
