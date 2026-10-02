//! Role: what an export writes, and what it refuses to write.
//! Position: `editing/commands` in the map engine.
//! Signals & state: explicit inputs only; every function here is pure over what it is handed.
//! Invariants: the compiled bytes are handed through UNCHANGED — a pretty re-parse is not byte-identical to the compiled route, and a download that differs from the server's answer is a trap. A clean compile gets the plain success message, never a celebratory zero.

/// Format the compact `/compiled` bytes for the Export Compiled download.
///
/// Returns the wire UTF-8 text **byte-identical** to `flatten_mod_document_json` / the compiled
/// route. Deliberately does **not** re-parse through `serde_json::Value` for a "pretty" download:
/// that round-trip is not byte-identical (whitespace), and without `preserve_order` it also
/// BTreeMap-sorts keys — the false "whitespace-only" comment on the old path set a trap for any
/// harness that compares against `GET /missions/:id/compiled`.
pub fn compiled_export_text(doc: &[u8]) -> Result<String, String> {
    String::from_utf8(doc.to_vec()).map_err(|e| format!("compiled document is not UTF-8: {e}"))
}

/// T-690 — the one-line author-facing summary of a compile's structured findings.
///
/// ## Why this replaces the toast rather than joining it
///
/// The compile used to say one of two things: "Downloaded the compiled mission document." or an
/// error. Everything it LEARNED — which authored values it discarded, and whose — was thrown away.
/// FNF v4's `init3DEN.sqf` is rated the single thing that framework does better than anyone else
/// precisely because Export is a build step there; TBD had the build step and none of the build
/// system. The findings now ride alongside the bytes
/// (`flatten::flatten_mod_document_json_with_diagnostics`) and are PUBLISHED to the T-655 validation
/// panel, which is the render surface and is not duplicated here. This string is only the pointer:
/// it names the count by severity so the toast says something true and finite, and sends the author
/// to the list rather than trying to be the list.
///
/// Empty findings → `None`: a clean compile gets the plain success message it always had, never a
/// celebratory "0 issues" (the panel's own empty-state doctrine).
///
/// Class-R / ungated so native `cargo test` can pin the wording without a browser (the
/// [`compiled_export_text`] precedent).
pub fn compile_diagnostics_summary(
    findings: &[crate::data::scenario::validate::Finding],
) -> Option<String> {
    use crate::data::scenario::validate::Severity;
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

/// Author-facing message when [`ROW_META`] never arrived.
///
/// `authenticated == false` means the session is missing/expired (hydrate 401 never sets the row);
/// do not tell the author to "save a version first" — that path would also 401.
pub fn row_meta_missing_message(authenticated: bool) -> &'static str {
    if authenticated {
        "This mission has no saved server row yet — save a version first, then export."
    } else {
        "Sign in to export the compiled mission — your session is missing or expired."
    }
}

/// **T-799 (a) — the once-per-gesture export latch, decided purely.**
///
/// The review's F-28/F-34 blob intercept caught **two** `createObjectURL` + two anchor clicks per
/// single "Export Compiled" click — the same 3213-byte payload emitted twice. The handler runs
/// once in code (`run_action` → `export_*_now` → [`download_json`] is one call), so the doubling is
/// DOM-level: the F-12 pointerup/click-pair family the T-785 input-model note names. A menu row that
/// removes itself mid-dispatch (the export dropdown closes on activation) is exactly the shape that
/// re-dispatches a synthesised second activation carrying the SAME `Event.timeStamp` as the first.
///
/// This is the guard the spec asks for: a latch keyed on **gesture identity**, not a blind time
/// debounce. `stamp` is the browser `Event.timeStamp` (`DOMHighResTimeStamp`, a `f64`), which is
/// identical across the events synthesised from one physical activation and distinct for a genuine
/// second click (a real second intent is milliseconds later with its own stamp — and MUST still
/// fire). `last` is the stamp of the activation this latch last let through.
///
/// Returns `true` when `stamp` is a duplicate of `last` (drop it). A `0.0` stamp — the value
/// `Event.timeStamp` reports when there is no live event (a chord-driven or programmatic export, and
/// the native/test path) — is NEVER treated as a duplicate: those do not double-activate through the
/// DOM, and coalescing two legitimately-distinct programmatic exports would be the "real second
/// intent" this guard must not eat. Pure, so the whole rule is pinned on the native `cargo test`
/// shell rather than only under a headless browser.
#[must_use]
pub fn export_gesture_is_duplicate(last: f64, stamp: f64) -> bool {
    stamp != 0.0 && stamp == last
}

/// **T-799 (b) — unify the JSON-envelope export's metadata onto the mission ROW.**
///
/// The [`compile_export`](map_engine_core::mission::compile::compile_export) envelope hard-codes
/// `gameMode: ""` and `maxPlayers: 0` (they are not in the editor CRDT it reads), so the downloaded
/// `mission-<id>.json` disagreed with the compiled document about the same mission: the review saw
/// JSON `maxPlayers: 0 / gameMode: ''` beside Compiled `playerRange [1,64]`. Those two fields have
/// ONE authority — the `missions` ROW, where the create dialog wrote them (`POST /missions/:id`,
/// hydrated into [`ROW_HYDRATE`] by [`set_row_meta`]) — and the compiled export already reads
/// `max_players` from that same row (`MissionMeta` → `flatten`). This patches the envelope so BOTH
/// exports source `maxPlayers`/`gameMode` from the row.
///
/// `version` is left as the caller set it (the current adopted semver — the latest SAVED version),
/// which is the third leg of the acceptance's `maxPlayers/gameMode/version` triple; `title` is left
/// as `compile_export` derived it (the LIVE doc title), which is the source the compiled export is
/// also moved onto (see [`export_compiled_now`]). One source per field, and both exports read it.
///
/// Pure over an owned `Value` so the projection is pinned natively. `max_players`/`game_mode` are
/// `None` only when the row never arrived — the export still downloads (the envelope is the
/// re-importable superset, not the mod document, so a missing row is not the refusal it is for
/// Export Compiled), and the fields keep `compile_export`'s defaults in that case.
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

/// **T-799 (b) — the LIVE doc title from `small_maps_json`, for the compiled-export override.**
///
/// The non-blank, trimmed `meta.title` out of `MissionDocCore::small_maps_json` — the SAME field
/// [`compile_export`](map_engine_core::mission::compile::compile_export) reads for the JSON export's
/// `title`, and the same non-blank/trim rule its `meta_title_nonblank` applies (a whitespace-only
/// title is not a title, so the caller keeps the row's — an override that blanked the compiled name
/// on a stray space would be a new bug). `None` when the doc carries no usable title, so the compiled
/// export falls back to the row title it already had rather than to `""`. Pure over the JSON string
/// so the extraction is pinned natively beside the projection it feeds.
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
