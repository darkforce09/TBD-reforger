//! Turning a picked mission document into something the versions route will accept.
//!
//! **Role:** the pure half of the dossier's upload panel — the size budget, the envelope
//! unwrapping, the JSON parse, the suggested next version, the failure wording, and the
//! human-readable preview of what the picked document would change.
//! **Position:** pure data, no view; the upload panel calls every function here.
//! **Signals & state:** none.
//! **Invariants:** the over-budget refusal runs on the picked file's size *before* a byte is read,
//! because this is a 32-bit build and a parsed mission tree is several times the size of its own
//! source text. The failure wording always keeps the server's findings list: a verdict with no
//! cause is not something an author can act on.

use super::mission_diff::{CollectionDelta, MissionDiff};
use serde_json::Value;

/// Largest mission document this browser upload will accept, in bytes.
///
/// A memory limit, not a policy one. Parsing a document costs several times its source text —
/// every JSON object carries map overhead — and this runs in a 32-bit tab that is also holding
/// the map editor. The server's own cap is much larger and stays the authority: a file under this
/// budget can still be refused with a 413, and that message is surfaced verbatim rather than
/// pre-empted here.
pub(super) const UPLOAD_MAX_BYTES: usize = 8388608;

/// Refuse an over-budget file before it is read; `None` means accept.
///
/// Names both numbers, because "too large" without them is unactionable: the author cannot tell
/// whether they need to trim one squad or that this door is closed to them entirely.
pub(super) fn oversize_refusal(bytes: usize) -> Option<String> {
    (bytes > UPLOAD_MAX_BYTES).then(|| {
        format!(
            "That document is {} — this browser upload accepts up to {}. A mission that large has \
             to be saved from the Mission Creator, which builds the payload in memory instead of \
             parsing a file.",
            crate::v2::apps::editor::mission_size::format_bytes(bytes),
            crate::v2::apps::editor::mission_size::format_bytes(UPLOAD_MAX_BYTES)
        )
    })
}

/// Name a JSON value's kind for an error message an author can act on.
pub(super) fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a true/false value",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Accept both shapes an author can plausibly have on disk, and return the editor payload the
/// API validates.
///
/// * An **export envelope** — anything carrying `exportFormatVersion`, which both exporters emit
///   unconditionally. Its `payload` is lifted out.
/// * A **bare editor payload** — passed through untouched.
///
/// Keyed on `exportFormatVersion` rather than "has a `payload` key": the editor payload schema
/// has no top-level `payload` property, so both tests happen to work today, but only the version
/// marker is something the producers promise. Guessing from shape is how a future top-level key
/// would silently start eating documents.
pub(super) fn unwrap_export_envelope(doc: Value) -> Result<Value, String> {
    let Value::Object(mut obj) = doc else {
        return Err(format!(
            "A mission document must be a JSON object; this file's top level is {}.",
            json_kind(&doc)
        ));
    };
    if !obj.contains_key("exportFormatVersion") {
        return Ok(Value::Object(obj));
    }
    match obj.remove("payload") {
        Some(payload @ Value::Object(_)) => Ok(payload),
        Some(other) => Err(format!(
            "This looks like an exported mission file, but its \"payload\" is {} rather than an \
             object, so there is no editor document inside it to upload.",
            json_kind(&other)
        )),
        None => Err(
            "This looks like an exported mission file, but it has no \"payload\" — there is no \
             editor document inside it to upload."
                .to_string(),
        ),
    }
}

/// Refuse a payload that reuses one slot id inside a callsign, naming the offenders.
///
/// The slot id is what binds a person to a seat, so a duplicate is not a cosmetic problem: the
/// server would accept the document and two slots would then answer to the same identity.
pub(super) fn check_duplicate_slot_ids_in_payload(payload: &Value) -> Result<(), String> {
    let Some(editor) = payload.get("editor").and_then(|e| e.as_object()) else {
        return Ok(());
    };
    let Some(squads) = editor.get("squads").and_then(|s| s.as_array()) else {
        return Ok(());
    };
    let mut callsign_seen: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    for squad in squads {
        let callsign = squad
            .get("callsign")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| squad.get("name").and_then(|v| v.as_str()))
            .unwrap_or("squad");
        let Some(slot_ids) = squad.get("slotIds").and_then(|v| v.as_array()) else {
            continue;
        };
        let seen = callsign_seen.entry(callsign.to_string()).or_default();
        for id_val in slot_ids {
            if let Some(id_str) = id_val.as_str() {
                if !seen.insert(id_str.to_string()) {
                    return Err(format!(
                        "Duplicate slot id \"{id_str}\" under callsign \"{callsign}\"."
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Parse a picked file into the editor payload to post, or the reason it cannot be one.
///
/// The syntax error is kept verbatim: `serde_json`'s own message ends in `at line L column C`,
/// which is the single most useful thing anyone can be told about a broken multi-megabyte
/// document, and the server's message for the same file carries no position at all.
pub(super) fn parse_uploaded_document(text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Err("That file is empty.".to_string());
    }
    let doc: Value =
        serde_json::from_str(text).map_err(|e| format!("That file is not valid JSON — {e}."))?;
    let payload = unwrap_export_envelope(doc)?;
    check_duplicate_slot_ids_in_payload(&payload)?;
    Ok(payload)
}

/// Suggested next version number: bump the patch of the mission's current version.
///
/// The versions route enforces real semantic versioning and answers 409 on a duplicate, so the
/// suggestion has to be both valid and unused, and a patch bump of the current tip is the only
/// value guaranteed to be neither of the mission's known-taken ones. Pre-release and build
/// metadata are dropped rather than carried — `1.2.3-rc1` bumps to `1.2.4` — because incrementing
/// inside a pre-release tag would be a guess about the author's release scheme.
pub(super) fn next_semver(current: Option<&str>) -> String {
    const FALLBACK: &str = "0.1.0";
    let Some(cur) = current else {
        return FALLBACK.to_string();
    };
    let core = cur.split(['-', '+']).next().unwrap_or("");
    let mut parts = core.split('.');
    let (Some(maj), Some(min), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return FALLBACK.to_string();
    };
    let (Ok(maj), Ok(min), Ok(patch)) =
        (maj.parse::<u64>(), min.parse::<u64>(), patch.parse::<u64>())
    else {
        return FALLBACK.to_string();
    };
    format!("{maj}.{min}.{}", patch + 1)
}

/// Turn a failed upload into `(headline, findings)`.
///
/// The versions route answers 400 with the exact list of everything wrong with the document, and
/// the client folds that list into the error string as extra lines. Collapsing it back to one
/// line is the failure this panel exists to avoid: "invalid mission payload" names a verdict, not
/// a cause. So the generic arm always returns the split rows, and the status-specific arms exist
/// only for failures that genuinely carry no findings and that the author does something
/// different about — pick another version number, use the editor, sign in, check the connection.
pub(super) fn upload_failure(
    status: u16,
    msg: Option<&str>,
    semver: &str,
) -> (String, Vec<String>) {
    let (head, rows) = crate::v2::core::api::client::split_error_lines(msg);
    let head = head.filter(|h| !h.trim().is_empty());
    match status {
        409 => (
            format!(
                "Version {semver} already exists on this mission. Versions are immutable — choose \
                 a different number."
            ),
            Vec::new(),
        ),
        // The server names its own limit in its refusal; echo that back rather than restating a
        // number this file would then have to keep in sync with the server's configuration.
        413 => (
            head.unwrap_or_else(|| "The server refused the document as too large.".to_string()),
            Vec::new(),
        ),
        401 => (
            "Your session expired — sign in again and re-pick the document.".to_string(),
            Vec::new(),
        ),
        0 => (
            "The upload could not reach the server. Nothing was saved; try again.".to_string(),
            Vec::new(),
        ),
        _ => match (&head, rows.len()) {
            (Some(h), 0) => (format!("Rejected ({status}): {h}"), rows),
            (Some(h), n) => (
                format!("Rejected ({status}): {h} — {n} problem(s) listed below"),
                rows,
            ),
            (None, _) => (format!("Upload failed ({status})."), rows),
        },
    }
}

/// What the picked document would do to this mission, in the author's vocabulary.
///
/// Bounded by construction: the per-collection counts are exact, and the named samples stop at
/// the comparison's own cap, so this stays O(1) in the size of the mission.
pub(super) fn diff_summary_lines(diff: &MissionDiff) -> Vec<String> {
    let mut out: Vec<String> = diff
        .fields
        .iter()
        .map(|f| format!("{}: {} → {}", f.label, f.from, f.to))
        .collect();
    for c in diff.changed_collections() {
        let mut parts: Vec<String> = Vec::new();
        for (n, word) in [
            (c.added, "added"),
            (c.removed, "removed"),
            (c.moved, "moved"),
            (c.edited, "edited"),
        ] {
            if n > 0 {
                parts.push(format!("{n} {word}"));
            }
        }
        out.push(format!(
            "{}: {} → {} ({})",
            c.label,
            c.a_rows,
            c.b_rows,
            parts.join(", ")
        ));
    }
    // Rows that could not be keyed are counted in the totals above but classified nowhere, so the
    // summary has to say so: silently dropping what it could not read would make this a check
    // reporting success over an input it never examined.
    let unreadable: usize = diff
        .collections
        .iter()
        .map(CollectionDelta::unreadable)
        .sum();
    if unreadable > 0 {
        out.push(format!(
            "{unreadable} row(s) have no usable id — they are counted in the totals above but \
             could not be matched to anything."
        ));
    }
    out
}
