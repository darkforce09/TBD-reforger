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

#[test]
fn mission_editor_clears_compile_findings_on_hydrate() {
    let ed = editor_live();
    assert!(
        ed.contains("validation_panel::clear_compile_findings()"),
        "T-761: MissionEditorPage must clear COMPILE_FINDINGS on hydrate so mission B cannot              inherit mission A's build report"
    );
    // Order: hydrate_from_server returns, THEN clear — the ticket's "one clear on editor
    // hydrate". A clear that runs only on a different path would leave the defect.
    let hydrate = ed
        .find("mission_hydrate::hydrate_from_server(")
        .expect("hydrate_from_server call");
    let clear = ed
        .find("validation_panel::clear_compile_findings()")
        .expect("clear_compile_findings call");
    assert!(
        clear > hydrate,
        "T-761: clear must run after hydrate_from_server returns (got clear@{clear} hydrate@{hydrate})"
    );
    // The clear sits in the same production body that awaits hydrate — not a decoy in a comment.
    let after = &ed[hydrate..];
    let await_at = after
        .find(".await")
        .expect("hydrate_from_server must be awaited");
    let clear_rel = after
        .find("validation_panel::clear_compile_findings()")
        .expect("clear after hydrate anchor");
    assert!(
        clear_rel > await_at,
        "T-761: clear_compile_findings must follow the hydrate .await"
    );
}
