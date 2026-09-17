use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

/// Fragment-assembled `world_layer_prefs` store-call needles. If any of these appear inside the
/// Mission Settings (document) dialog, the editor-local half did not actually move.
fn store_call_needles() -> Vec<String> {
    vec![
        format!("save{}", "_prefs"),
        format!("save{}", "_basemap_view"),
        format!("apply{}", "_basemap_view"),
        format!("refresh{}", "_world_layers"),
        format!("world{}", "_layer_prefs"),
    ]
}

/// Separation pin (document half): after the move, no world-layer/basemap store call survives in
/// `render_prefs_section` — the render-pref block that stays in Mission Settings holds only the
/// hillshade/grid document keys and the pointer row. Perturbation that this catches: leaving (or
/// pasting back) the basemap buttons or the 12 layer toggles into Mission Settings.
#[test]
fn mission_settings_render_prefs_holds_no_world_layer_toggles() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let body = only_body(&src, &format!("fn render{}", "_prefs_section"));
    for needle in store_call_needles() {
        assert!(
            !body.contains(&needle),
            "T-691: `{needle}` must not remain in render_prefs_section — the editor-local \
             basemap/world-layer controls moved to EditorPreferencesDialog"
        );
    }
    // The document keys it DOES keep: hillshade + grid still author through the env gate.
    let author = format!("author{}", "_env");
    assert!(
        body.contains(&author),
        "T-691: render_prefs_section must still author the hillshade/grid document keys"
    );
}

/// Separation pin (document half, dialog scope): the whole Mission Settings dialog body carries
/// no world-layer store call either (guards against the toggles being reintroduced directly in
/// the component rather than via the helper).
#[test]
fn mission_settings_dialog_body_holds_no_store_calls() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let body = only_body(&src, &format!("fn Mission{}", "SettingsDialog"));
    for needle in store_call_needles() {
        assert!(
            !body.contains(&needle),
            "T-691: `{needle}` must not appear in MissionSettingsDialog — editor-local prefs \
             live only in EditorPreferencesDialog"
        );
    }
}

/// Separation pin (editor-local half): EditorPreferencesDialog's content contains NO
/// `author_env` write — every control there is a localStorage editor preference, never a
/// mission-document key. Perturbation this catches: wiring a hillshade/grid (or any
/// document-key) control into the preferences body. The content lives in
/// `render_editor_prefs_body`, sliced out here.
#[test]
fn editor_preferences_dialog_writes_no_author_env() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let body = only_body(&src, &format!("fn render{}", "_editor_prefs_body"));
    let author = format!("author{}", "_env");
    assert!(
        !body.contains(&author),
        "T-691: EditorPreferencesDialog must contain no `{author}` — it is the editor-local, \
         per-user surface, not a mission-document editor"
    );
    // And it MUST carry the moved editor-local controls (the move actually happened).
    assert!(
        body.contains(&format!("save{}", "_basemap_view"))
            && body.contains(&format!("save{}", "_prefs")),
        "T-691: the basemap + world-layer store writes must live in the preferences body"
    );
}

/// Dialog-opens pin: the opener is wired end to end without leaving `owns`. The pointer row in
/// `render_prefs_section` calls `open_editor_preferences`; that fn arms the parked signal; and
/// `MissionSettingsDialog` both registers the signal (`set_prefs_signal`) and mounts the
/// preferences dialog on it. Perturbation this catches: dropping the mount, the registration, or
/// the pointer-row call.
#[test]
fn editor_preferences_opener_is_wired() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let opener = format!("open{}", "_editor_preferences");
    let mount = format!("Editor{}", "PreferencesDialog");
    let register = format!("set{}", "_prefs_signal");

    // (a) the opener fn arms a boolean signal to true.
    let opener_body = only_body(&src, &format!("fn {opener}"));
    assert!(
        opener_body.contains(".set(true)"),
        "T-691: {opener} must set the parked open signal to true"
    );

    // (b) the pointer row in the document dialog calls the opener.
    let prefs_body = only_body(&src, &format!("fn render{}", "_prefs_section"));
    assert!(
        prefs_body.contains(&format!("{opener}()")),
        "T-691: the Mission Settings pointer row must call {opener}()"
    );

    // (c) MissionSettingsDialog registers the signal and mounts the preferences dialog on it.
    let dlg_body = only_body(&src, &format!("fn Mission{}", "SettingsDialog"));
    assert!(
        dlg_body.contains(&register),
        "T-691: MissionSettingsDialog must register the prefs signal via {register}"
    );
    assert!(
        dlg_body.contains(&mount) && dlg_body.contains("prefs_open"),
        "T-691: MissionSettingsDialog must mount {mount} bound to prefs_open"
    );

    // (d) the preferences dialog is a real component that gates on its `open` signal.
    let comp_body = only_body(&src, &format!("fn {mount}"));
    assert!(
        comp_body.contains("open.get()"),
        "T-691: {mount} must render no DOM while closed (gate on open.get())"
    );
}
