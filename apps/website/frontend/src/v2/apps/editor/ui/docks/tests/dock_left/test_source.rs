//! Reassembles the live left-dock modules for source-inspection tests.

use std::sync::OnceLock;

/// Return live production source with the view fragments at their call sites.
pub(super) fn dock_left_source() -> &'static str {
    static SOURCE: OnceLock<String> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let root = include_str!("../../dock_left.rs");
        let root = root.split("#[cfg(test)]").next().unwrap_or(root);
        let mut view = include_str!("../../dock_left/view.rs").to_string();
        for (name, fragment) in [
            (
                "places_body",
                include_str!("../../dock_left/view/places_body.rs"),
            ),
            (
                "full_dock",
                include_str!("../../dock_left/view/full_dock.rs"),
            ),
        ] {
            let invocation = view
                .lines()
                .find(|line| line.contains(&format!("{name}!(")))
                .expect("view fragment invocation remains present")
                .to_string();
            let mut expanded = fragment.to_string();
            for binding in [
                "nodes",
                "selected",
                "active_layer",
                "collapsed",
                "layer_query",
                "layer_nodes",
                "layers_filtered_empty",
                "doc_hits",
                "sel_facets",
                "tab",
                "query",
                "places",
                "places_armed",
                "bookmarks",
                "renaming",
                "rename_draft",
                "rename_abandon",
                "adding",
                "add_draft",
                "commit_bookmarks",
                "arm_places",
                "tab_btn",
                "fly_row",
                "hits_body",
                "facets_row",
                "places_body",
                "full",
                "stub",
            ] {
                expanded = expanded.replace(&format!("${binding}"), binding);
            }
            view = view.replace(&invocation, &expanded);
        }
        [
            root,
            &view,
            include_str!("../../dock_left/places.rs"),
            include_str!("../../dock_left/bookmarks.rs"),
            include_str!("../../dock_left/camera.rs"),
            include_str!("../../dock_left/document_search.rs"),
        ]
        .join("\n")
    })
}
