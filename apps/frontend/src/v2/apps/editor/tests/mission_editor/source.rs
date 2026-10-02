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

pub(super) fn live_pointer_gestures() -> String {
    let mut source = live_code(include_str!("../../input/pointer_gestures.rs"));
    source.push('\n');
    source.push_str(&live_pointer_gesture_handlers());
    source
}

pub(super) fn live_pointer_gesture_handlers() -> String {
    [
        include_str!("../../input/pointer_gestures/wheel_zoom.rs"),
        include_str!("../../input/pointer_gestures/pointer_down.rs"),
        include_str!("../../input/pointer_gestures/pointer_move.rs"),
        include_str!("../../input/pointer_gestures/pointer_up.rs"),
        include_str!("../../input/pointer_gestures/pointer_up/special_drag_release.rs"),
        include_str!("../../input/pointer_gestures/context_menu.rs"),
        include_str!("../../input/pointer_gestures/double_click.rs"),
    ]
    .into_iter()
    .map(live_code)
    .collect::<Vec<_>>()
    .join("\n")
}

pub(super) fn live_document_history() -> String {
    [
        include_str!("../../bridge/document_host/history.rs"),
        include_str!("../../bridge/document_host/history/render_lanes.rs"),
    ]
    .into_iter()
    .map(live_code)
    .collect::<Vec<_>>()
    .join("\n")
}
