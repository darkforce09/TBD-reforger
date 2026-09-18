//! Compilation browser commands.
use super::*;

/// ** download the document the game server will actually receive.**
///
/// Returns the compact compiled mod document (byte-identical to `GET /compiled`'s body when the
/// local doc matches the saved version), or a message fit to show an author.
///
/// ## Why this exists at all
///
/// `GET /missions/:id/compiled` takes a `ServiceAuth` (`missions::handlers::mission_export::get_compiled_mission`)
/// it answers game servers, not browsers — so an author has no way to fetch it. Until this,
/// "Export JSON" downloaded [`compile_export`]'s `MissionExport` envelope: the editor SUPERSET,
/// `{exportFormatVersion, missionId, title, …, payload}`, whose `payload` is the editor graph. That
/// is the right file for re-importing into the editor and it is **not** the mod document  it has
/// no `slots[]`, no `orbat`, no `radioPlan`, no `winConditions`, and the mod cannot load it. So
/// there was no way to see the compiled document before a game server did.
///
/// ## Why the answer can be trusted
///
/// This runs `flatten_mod_document_json`  the same `map-engine-core` compile `/compiled` runs,
/// over the same two inputs:
///
///   * the **row**, from `GET /missions/:id` ([`ROW_META`]), which is where the server gets
///     `author`, `maxPlayers` and the fallback time/weather;
///   * the **save-shaped payload** (`include_orbat = false`)  byte-for-byte what
///     `POST /missions/:id/versions` stores and therefore what `/compiled` later reads. `orbat` is
///     omitted for the same reason the save omits it: the flatten derives its own from `editor`,
///     and including it would put a key in the preview's input that the stored version never has.
///
/// Their agreement is not asserted here  it is pinned natively, on both halves, by
/// `website-api`'s `client_twin_is_byte_identical_to_the_compiled_route` (the compile) and
/// `dto::r_api::compiled_meta_is_the_row_the_server_compiles_from` (the row).
///
/// ## The one honest difference, and it is the point
///
/// `/compiled` serves the last **saved** version; this compiles the document **as it is now**,
/// unsaved edits included. That is what makes it useful  you can see what a save would ship
/// before shipping it  but it means a dirty document previews something the server does not yet
/// have. The caller says so in the toast rather than hiding it.
///
/// # Errors
/// Returns a display message when the row never arrived (local-only id, 404, offline, **401 /
/// expired session** see [`ROW_META`] + [`row_meta_missing_message`]), when the editor is not
/// mounted, or when the compile refuses (no placed slots is the common one, and it is the same
/// `409` a game server would get).
pub fn compiled_document_json() -> Result<String, String> {
    compiled_document_json_with_diagnostics().map(|(text, _)| text)
}

/// the compile, with the structured result it produced ALONGSIDE the bytes.
///
/// This is the body [`compiled_document_json`] projects: one compile, one document, one finding
/// list. Splitting it the other way round (a second compile just for the findings) is the shape
/// `flatten_mod_document_json_full` exists to forbid  two compiles are two things that can
/// disagree about what was compiled.
///
/// The findings are `map_engine_core::mission::validate::Finding`s  the  vocabulary
/// (`rule_id` / `severity` / `primitive` / `message` / `subject` / `subject_id`), reused so the
///  panel renders a compile finding through exactly the same row as a validation finding
/// and click-to-select works on both.
///
/// # Errors
/// Same three refusals as [`compiled_document_json`]  a missing row, an unmounted editor, or a
/// compile that produced no document. A FINDING is never one of them.
pub fn compiled_document_json_with_diagnostics() -> Result<CompiledWithDiagnostics, String> {
    let Some(snap) = snapshot() else {
        return Err("Editor not ready".to_string());
    };
    let authenticated = snap.auth.access_token.get_untracked().is_some()
        && snap.auth.user.get_untracked().is_some();
    let Some(mut meta) = ROW_META.with(|r| r.borrow().as_ref().map(clone_meta)) else {
        return Err(row_meta_missing_message(authenticated).to_string());
    };
    // Carry the mission id from the route when the row's is blank, matching the envelope export's
    // `meta.id`-then-route fallback (`compile_export`). `mission_doc_id` in the flatten normalizes
    // whatever lands here into the schema's id space either way.
    if meta.id.is_empty() {
        meta.id = snap.mission_id.clone();
    }
    // the compiled export's title read from `ROW_META`, which is the LIBRARY row's
    // title (`set_row_meta`, refreshed only by `GET /missions/:id`). Retitling in the editor
    // writes the LIVE doc (`meta.title` in `small_maps_json`) but not the row, so the two exports
    // named the same mission differently  the RowMirror gap the review caught (F-34). One source:
    // both exports use the live doc title. The compile-time override here (rather than a title
    // RowMirror PATCH) is the alternative the review offered, and it needs no wire round-trip, so
    // it agrees the instant the author types  no PATCH to confirm (the  lesson), and it does
    // not fork the / RowMirror (which stays time+weather; title never joins it). The
    // JSON export already reads its title from this same `snap.small`, via `compile_export`.
    if let Some(live_title) = super::super::live_doc_title(&snap.small) {
        meta.title = live_title;
    }
    let payload = compile_payload(&snap.small, &snap.slots, false);
    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let meta_bytes = serde_json::to_vec(&meta).map_err(|e| e.to_string())?;

    let (doc, findings) = flatten_mod_document_json_with_diagnostics(&meta_bytes, &payload_bytes)?;
    // ship the compact wire bytes (byte-identical to `/compiled`). Do not re-parse to
    // `serde_json::Value` for a "pretty" download  that is not whitespace-only vs the route.
    compiled_export_text(&doc).map(|text| (text, findings))
}

/// `MissionMeta` is a plain data carrier in core and deliberately not `Clone` (it is an input type
/// built once per compile); this is the local copy out of the `thread_local` so no borrow is held
/// across the compile below.
pub(super) fn clone_meta(m: &MissionMeta) -> MissionMeta {
    MissionMeta {
        id: m.id.clone(),
        title: m.title.clone(),
        author: m.author.clone(),
        terrain: m.terrain.clone(),
        custom_terrain_name: m.custom_terrain_name.clone(),
        max_players: m.max_players,
        time_of_day: m.time_of_day.clone(),
        weather_preset: m.weather_preset.clone(),
    }
}
