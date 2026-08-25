use crate::editor::arsenal::class_r_scrub::live_code;

fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
}

/// The panel is mounted as a real component, fed the `doc_tick` re-eval channel (the T-666
/// doc-change tick), and its payload source is registered from the wasm mount.
#[test]
fn the_validation_panel_is_mounted_and_wired_to_doc_tick() {
    let ed = editor_live();
    assert!(
        ed.contains("validation_panel::ValidationPanel doc_tick"),
        "T-655: the ValidationPanel must be mounted with the doc_tick re-eval channel"
    );
    assert!(
        ed.contains("validation_panel::register_payload_source("),
        "T-655: the panel's compiled-payload source must be registered from the wasm mount"
    );
    assert!(
        ed.contains("validation_panel::register_select_by_id("),
        "T-655: the click-to-select router (subject_id → selection) must be registered from the \
         wasm mount, where the doc/selection/engine handles live"
    );
    // The router routes through the SAME selection seam the rest of the editor uses (engine
    // set_selection + centre + refresh_selection), keyed on the finding's subject_id.
    assert!(
        ed.contains("e.set_selection(ids)") && ed.contains("mission_history::refresh_selection()"),
        "T-655: click-to-select must replace the selection + refresh mirrors (the open_attributes \
         seam), not a bespoke path"
    );
    // The registered source compiles the SAVE-shape payload (the editor.{factions,squads,slots}
    // block the rules read) and threads the T-658 known-asset-id catalogue.
    assert!(
        ed.contains("compile::compile_payload(") && ed.contains("known_asset_ids_from_registry("),
        "T-655/T-658: the source must feed compile_payload + the known-asset-id catalogue"
    );
}

/// Hide-chrome survival + always-on: the panel mount is OUTSIDE every `chrome_hidden` gate (a
/// Backspace hide-interface leaves it visible — correctness diagnostics are never gated, T-635's
/// doctrine), and it is not behind any debug flag. Proven by locating the mount and checking that
/// no `chrome_hidden` gate (nor a `debug_hud` gate) opens between the ungated-dialog landmark
/// (the context-menu overlay, the same landmark the T-647 picker pin uses) and it.
#[test]
fn the_validation_panel_survives_hide_chrome_and_is_always_on() {
    let ed = editor_live();
    let mount = ed
        .find("validation_panel::ValidationPanel doc_tick")
        .expect("T-655: the ValidationPanel mount");
    let landmark = ed
        .find("ContextMenuOverlay menu=")
        .expect("context menu mount is the ungated-dialog landmark");
    assert!(
        mount > landmark,
        "T-655: the panel must mount after the ungated-dialog landmark"
    );
    let between = &ed[landmark..mount];
    assert!(
        !between.contains("(!chrome_hidden.get()).then("),
        "T-655: the panel is DIAGNOSTICS — no chrome_hidden gate may sit between the ungated \
         dialogs and its mount (it survives Backspace hide-chrome, T-635 doctrine)"
    );
    // Always-on: not gated behind the telemetry HUD debug flag either.
    assert!(
        !between.contains("debug_hud_shown.get()") && !between.contains("debug_hud.get()"),
        "T-655: validation is ALWAYS ON — the panel must not sit behind a debug flag"
    );
}
