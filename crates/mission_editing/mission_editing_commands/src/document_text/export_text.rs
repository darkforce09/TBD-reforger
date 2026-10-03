//! **Role:** what an export writes, and what it refuses to write.
//! **Position:** `document_text::export_text` in `mission_editing_commands`.
//! **Signals & state:** explicit inputs only; every function here is pure over what it is handed.
//! **Invariants:** the compiled bytes are handed through UNCHANGED — a pretty re-parse is not
//! byte-identical to the compiled route, and a download that differs from the server's answer is a
//! trap. A clean compile gets the plain success message, never a celebratory zero.

use crate::error::Result;

/// Format the compact `/compiled` bytes for the Export Compiled download.
///
/// Returns the wire UTF-8 text **byte-identical** to the compiled route
/// (`GET /missions/:id/compiled`). It never re-parses through `serde_json::Value` for a "pretty"
/// download: that round-trip is not byte-identical (whitespace), and without `preserve_order` it
/// also sorts keys, so a harness comparing the download against the route would fail.
///
/// # Errors
///
/// [`crate::Error::CompiledDocumentNotUtf8`] when the compiled bytes are not UTF-8.
pub fn compiled_export_text(doc: &[u8]) -> Result<String> {
    Ok(String::from_utf8(doc.to_vec())?)
}

/// The one-line author-facing summary of a compile's structured findings.
///
/// The compile's findings ride alongside the compiled bytes and are published to the validation
/// panel, which is their render surface. This string is only the pointer: it names the count by
/// severity, so the toast says something true and finite and sends the author to the list rather
/// than trying to be the list.
///
/// Empty findings → `None`: a clean compile gets the plain success message, never a celebratory
/// "0 issues" (the panel's own empty-state rule). Pure, so native tests pin the wording.
pub fn compile_diagnostics_summary(findings: &[mission_validation::Finding]) -> Option<String> {
    use mission_validation::Severity;
    if findings.is_empty() {
        return None;
    }
    let count = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    let label = |n: usize, noun: &str| {
        if n == 1 {
            format!("1 {noun}")
        } else {
            format!("{n} {noun}s")
        }
    };
    let mut parts: Vec<String> = Vec::new();
    for (sev, noun) in [
        (Severity::Error, "error"),
        (Severity::Warning, "warning"),
        (Severity::Info, "note"),
    ] {
        let n = count(sev);
        if n > 0 {
            parts.push(label(n, noun));
        }
    }
    Some(format!(
        "The compile reported {} — see the validation panel.",
        parts.join(" · ")
    ))
}

/// Author-facing message when the mission row's metadata never arrived.
///
/// `authenticated == false` means the session is missing or expired (a hydrate answered 401 and
/// never set the row); the message does not tell the author to "save a version first", because
/// that path would answer 401 too.
pub fn row_meta_missing_message(authenticated: bool) -> &'static str {
    if authenticated {
        "This mission has no saved server row yet — save a version first, then export."
    } else {
        "Sign in to export the compiled mission — your session is missing or expired."
    }
}

/// **The once-per-gesture export latch, decided purely.**
///
/// A menu row that removes itself mid-dispatch (the export dropdown closes on activation) makes the
/// browser re-dispatch a synthesised second activation carrying the SAME `Event.timeStamp` as the
/// first, which would download the same compiled document twice. The latch is keyed on **gesture
/// identity**, not a blind time debounce: `stamp` is the browser `Event.timeStamp`
/// (`DOMHighResTimeStamp`, an `f64`), identical across the events synthesised from one physical
/// activation and distinct for a genuine second click (a real second intent carries its own stamp
/// and MUST still fire). `last` is the stamp of the activation this latch last let through.
///
/// Returns `true` when `stamp` is a duplicate of `last` (drop it). A `0.0` stamp, which
/// `Event.timeStamp` reports when there is no live event (a chord-driven or programmatic export, and
/// the native test path), is NEVER a duplicate: those do not double-activate through the DOM, and
/// coalescing two distinct programmatic exports would eat a real second intent. Pure, so the whole
/// rule is pinned by native tests rather than only under a headless browser.
#[must_use]
pub fn export_gesture_is_duplicate(last: f64, stamp: f64) -> bool {
    stamp != 0.0 && stamp == last
}

/// **Source the JSON-envelope export's metadata from the mission ROW.**
///
/// The envelope compiler (`mission_payload`'s export envelope) writes `gameMode: ""` and
/// `maxPlayers: 0`, because those fields are not in the editor document it reads. They have ONE
/// authority, the `missions` row the create dialog wrote (`POST /missions/:id`, hydrated into the
/// Mission Creator's row metadata), and the compiled export already reads `max_players` from that
/// same row. This patches the envelope so BOTH exports source `maxPlayers` and `gameMode` from the
/// row.
///
/// `version` is left as the caller set it (the latest saved version); `title` is left as the
/// envelope compiler derived it (the live document title, which the compiled export also uses
/// through [`live_doc_title`]). One source per field, and both exports read it.
///
/// Pure over an owned `Value`. `max_players` and `game_mode` are `None` only when the row never
/// arrived: the export still downloads (the envelope is the re-importable superset, not the mod
/// document, so a missing row is not a refusal here), and the fields keep the envelope's defaults.
pub fn apply_row_metadata_to_export(
    mut envelope: serde_json::Value,
    max_players: Option<i64>,
    game_mode: Option<&str>,
) -> serde_json::Value {
    if let Some(obj) = envelope.as_object_mut() {
        if let Some(mp) = max_players {
            obj.insert("maxPlayers".to_string(), serde_json::json!(mp));
        }
        if let Some(gm) = game_mode {
            obj.insert("gameMode".to_string(), serde_json::json!(gm));
        }
    }
    envelope
}

/// **The live document title, for the compiled-export override.**
///
/// The non-blank, trimmed `meta.title` out of `MissionDocCore::small_maps_json`: the SAME field the
/// envelope compiler reads for the JSON export's `title`, under the same non-blank and trim rule (a
/// whitespace-only title is not a title, so the caller keeps the row's). `None` when the document
/// carries no usable title, so the compiled export falls back to the row title rather than to `""`.
/// Pure over the JSON string, so the extraction is pinned natively beside the projection it feeds.
#[must_use]
pub fn live_doc_title(small_maps_json: &str) -> Option<String> {
    let small: serde_json::Value = serde_json::from_str(small_maps_json).ok()?;
    small
        .get("meta")
        .and_then(|m| m.get("title"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
#[path = "tests/export_text.rs"]
mod tests;
