//! Exports browser commands.
use super::*;

/// ** open one export gesture; `true` when it is a NEW one that should run.**
///
/// The activation-site guard for the F-28/F-34 double export. `stamp` is the click's
/// `Event.timeStamp` (the caller passes `ev.time_stamp()` from the export row's `on:click`).
/// A duplicate  the second, synthesised activation of the SAME physical click, which carries
/// the SAME stamp  returns `false` and is dropped by the caller; a genuinely new gesture (a
/// different, later stamp) records itself as the latch and returns `true`. See
/// [`super::super::export_gesture_is_duplicate`] for the pure rule and why a `0.0` stamp always passes.
///
/// Deliberately at the ONE seam both export rows funnel through in the strip, not inside
/// `export_now` / `export_compiled_now`: the smoke bridge and any future non-DOM caller reach
/// those two directly with no `Event` to key on, and must not be latched (a `0.0` stamp would
/// pass anyway, but keeping the guard at the DOM edge keeps the export bodies unconditional).
pub fn begin_export_gesture(stamp: f64) -> bool {
    let last = LAST_EXPORT_STAMP.with(std::cell::Cell::get);
    if super::super::export_gesture_is_duplicate(last, stamp) {
        return false;
    }
    LAST_EXPORT_STAMP.with(|c| c.set(stamp));
    true
}

/// Trigger the server-truth download and report the outcome ().
///
/// ** this is where the compile stops being a pass/fail.** The compile now returns a
/// structured finding list alongside the bytes; this publishes that list to the  validation
/// panel ([`crate::v2::apps::editor::ui::inspector::validation_panel::publish_compile_findings`]) and lets the toast shrink back
/// to what a toast is good at  a one-line verdict with a pointer. The panel is the render
/// surface and is deliberately not duplicated here.
///
/// The publish happens even when the list is EMPTY, and that is load-bearing: a clean compile
/// must CLEAR the previous compile's findings, or the panel would show a stale build report
/// after the author fixed everything in it.
///
/// `toasts` is resolved at component setup by the caller  `use_toasts()` is an `expect_context`
/// and would panic from a DOM handler, the `RowMirror` precedent.
pub fn export_compiled_now(toasts: crate::v2::core::ui::toast::Toasts) {
    let mission_id = EDITOR_CTX
        .with(|c| c.borrow().as_ref().map(|ctx| ctx.mission_id.clone()))
        .unwrap_or_default();
    match compiled_document_json_with_diagnostics() {
        Ok((json, findings)) => {
            let filename = format!("mission-{mission_id}.compiled.json");
            if let Err(e) = download_json(&filename, &json) {
                toasts.error(format!("Could not start the download: {e:?}"));
                return;
            }
            // The findings reach the panel through the engine's own row type, so a compile
            // finding renders  and click-to-selects on its `subject_id` — exactly like a
            // validation finding. Published AFTER the download starts: a diagnostic is not a
            // refusal, and the file the author asked for is not held back by one.
            let summary = super::super::compile_diagnostics_summary(&findings);
            crate::v2::apps::editor::ui::inspector::validation_panel::publish_compile_findings(
                    findings
                        .iter()
                        .map(crate::v2::apps::editor::ui::inspector::validation_panel::PanelFinding::from_finding)
                        .collect(),
                );
            // Naming the staleness is the whole reason this is a toast and not a silent download:
            // the file is the CURRENT document, which is only what a game server would fetch once
            // this state is saved.
            let staleness = if crate::v2::apps::editor::bridge::document_host::history::is_dirty() {
                Some(
                    "Downloaded the compiled mission document — compiled from your unsaved \
                         changes, so the server still serves the last saved version.",
                )
            } else {
                None
            };
            match (staleness, summary) {
                (Some(stale), Some(s)) => toasts.message(format!("{stale} {s}")),
                (Some(stale), None) => toasts.message(stale.to_string()),
                (None, Some(s)) => {
                    toasts.message(format!("Downloaded the compiled mission document. {s}"))
                }
                (None, None) => toasts.success("Downloaded the compiled mission document."),
            }
        }
        Err(e) => toasts.error(e),
    }
}

/// Export the current mission as a downloaded `mission-<id>.json` (React `exportJson`): compile with
/// `orbat` included, wrap in the `MissionExport` envelope, pretty-print, and trigger the browser
/// download. `version` is the current semver (envelope `version` field).
///
/// **This is the editor SUPERSET, not the mod document** it round-trips back into the editor and
/// the mod cannot load it. The compiled document an author ships is
/// [`export_compiled_now`] (); both are kept because they answer different questions.
pub fn export_now(version: &str) {
    let Some(snap) = snapshot() else {
        return;
    };
    let payload = compile_payload(&snap.small, &snap.slots, true);
    let doc = compile_export(
        &payload,
        &snap.small,
        &snap.mission_id,
        version,
        &js_date_iso(),
    );
    // `compile_export` hard-codes `gameMode: ""` / `maxPlayers: 0` (they are not in
    // the editor CRDT it reads). Source them from the mission ROW instead  the same place the
    // create dialog wrote and the same place Export Compiled reads `max_players`  so the two
    // exports stop disagreeing about the same mission. `version` (the latest saved semver) and
    // `title` (the live doc title, which `compile_export` already read from `snap.small`) are the
    // other two legs both exports now agree on; Export Compiled is moved onto the live title in
    // `compiled_document_json_with_diagnostics`.
    let game_mode = hydrated_row().map(|r| r.game_mode);
    let doc =
        super::super::apply_row_metadata_to_export(doc, row_max_players(), game_mode.as_deref());
    let json = serde_json::to_string_pretty(&doc).unwrap_or_default();
    let filename = format!("mission-{}.json", snap.mission_id);
    let _ = download_json(&filename, &json);
}
