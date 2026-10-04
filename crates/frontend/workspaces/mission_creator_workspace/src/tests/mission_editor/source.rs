//! Source assembly for tests that inspect the split editor lifecycle.

use mission_creator_engine_bridge::test_support::production_half;

use frontend_test_support::class_r_scrub::{live_code, only_body};

pub(super) fn raw_editor() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let page = include_str!("../../mission_editor.rs");
            let page = production_half(page);
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
            source.push_str(frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/frontend/workspaces/mission_creator_state/src/armed_place.rs",
            ));
            source.push_str(frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/frontend/workspaces/mission_creator_state/src/transform.rs",
            ));
            source.push_str("\n#[cfg(test)]\n");
            source
        })
        .as_str()
}

pub(super) fn live_pointer_gestures() -> String {
    let mut source = live_code(frontend_test_support::repository_root::repository_text(
        env!("CARGO_MANIFEST_DIR"),
        "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures.rs",
    ));
    source.push('\n');
    source.push_str(&live_pointer_gesture_handlers());
    source
}

pub(super) fn live_pointer_gesture_handlers() -> String {
    [
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/wheel_zoom.rs",
        ),
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/pointer_down.rs",
        ),
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/pointer_move.rs",
        ),
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/pointer_up.rs",
        ),
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/pointer_up/special_drag_release.rs",
        ),
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/context_menu.rs",
        ),
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/double_click.rs",
        ),
    ]
    .into_iter()
    .map(live_code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// The block a gesture handler factory hands to `Closure::new`, found inside the only
/// `fn <factory>(` of `src`.
///
/// Each handler file under `input/pointer_gestures/` builds exactly one closure, which the factory
/// returns as its tail expression, so this is the event handler's own code: the factory's handle
/// clones before it are excluded, and a second `Closure::` in the factory fails the lookup.
pub(super) fn gesture_closure_body<'a>(src: &'a str, factory: &str) -> &'a str {
    only_body(only_body(src, &format!("fn {factory}(")), "Closure::")
}

pub(super) fn live_document_history() -> String {
    [
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/history.rs",
        ),
        frontend_test_support::repository_root::repository_text(
            env!("CARGO_MANIFEST_DIR"),
            "crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/history/render_lanes.rs",
        ),
    ]
    .into_iter()
    .map(live_code)
    .collect::<Vec<_>>()
    .join("\n")
}
