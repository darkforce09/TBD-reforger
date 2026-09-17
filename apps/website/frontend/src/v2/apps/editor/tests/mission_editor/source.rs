//! Source assembly for tests that inspect the split editor lifecycle.

use crate::v2::core::test_support::class_r_scrub::live_code;

pub(super) fn raw_editor() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let page = include_str!("../../mission_editor.rs");
            let page = page.split("\n#[cfg(test)]").next().expect("page source");
            let mut source = page.to_string();
            source.push_str(include_str!("../../mission_editor/canvas_mount.rs"));
            source.push_str(include_str!(
                "../../mission_editor/canvas_mount/boot_tasks.rs"
            ));
            source.push_str(include_str!(
                "../../mission_editor/canvas_mount/document_setup.rs"
            ));
            source.push_str(include_str!(
                "../../mission_editor/canvas_mount/input_listeners.rs"
            ));
            source.push_str(include_str!(
                "../../mission_editor/canvas_mount/registry_effects.rs"
            ));
            source.push_str(include_str!("../../mission_editor/page_effects.rs"));
            source.push_str(include_str!("../../mission_editor/document_helpers.rs"));
            source.push_str(include_str!("../../mission_editor/registry_loading.rs"));
            source.push_str(include_str!("../../mission_editor/toolbar_dispatch.rs"));
            source.push_str(include_str!("../../mission_editor/armed_place.rs"));
            source.push_str(include_str!("../../mission_editor/transform.rs"));
            source.push_str("\n#[cfg(test)]\n");
            source
        })
        .as_str()
}

pub(super) fn live_editor() -> String {
    let source = raw_editor();
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    assert_eq!(
        source.matches(&anchor).count(),
        1,
        "editor page anchor must be unique"
    );
    live_code(&source[source.find(&anchor).expect("counted above")..])
}
