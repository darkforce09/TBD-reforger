//! Reassembles the live top-strip modules for source-inspection tests.

use std::sync::OnceLock;

/// Return the live production source with view fragments at their call sites.
pub(super) fn top_strip_source() -> &'static str {
    static SOURCE: OnceLock<String> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let root = include_str!("../../top_strip.rs");
        let root = root.split("#[cfg(test)]").next().unwrap_or(root);
        let mut view = include_str!("../../top_strip/view.rs").to_string();
        for (name, fragment) in [
            ("menu_row", include_str!("../../top_strip/view/menu_row.rs")),
            ("tool_row", include_str!("../../top_strip/view/tool_row.rs")),
            ("overlays", include_str!("../../top_strip/view/overlays.rs")),
        ] {
            let invocation = view
                .lines()
                .find(|line| line.contains(&format!("{{{name}!(")))
                .expect("view fragment invocation remains present")
                .to_string();
            let mut expanded = fragment.to_string();
            for binding in [
                "title",
                "can_undo",
                "can_redo",
                "save_semver",
                "save_status",
                "dirty",
                "settings_open",
                "doc_tick",
                "obj_count",
                "orbat_open",
                "open_menu",
                "export_open",
                "save_open",
                "save_notes",
                "validation_open",
                "hint_open",
                "set_hint",
                "close_transients",
                "transient_closer_id",
                "row_mirror",
                "toasts",
                "save_findings",
                "last_flush",
                "recency_tick",
                "save_was_open",
                "env",
                "census",
                "summary",
                "validation_findings",
                "export_gesture_ok",
                "run_action",
                "widget_is",
                "snap_on",
                "title_fallback",
            ] {
                expanded = expanded.replace(&format!("${binding}"), binding);
            }
            view = view.replace(&invocation, &expanded);
        }
        [
            root,
            include_str!("../../top_strip/arrange.rs"),
            include_str!("../../top_strip/menu_catalog.rs"),
            include_str!("../../top_strip/clock_and_draft.rs"),
            include_str!("../../top_strip/row_mirror.rs"),
            include_str!("../../top_strip/dialog_focus.rs"),
            &view,
            include_str!("../../top_strip/mission_summary.rs"),
        ]
        .join("\n")
    })
}
