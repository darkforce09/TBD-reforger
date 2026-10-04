use super::*;

mod compositions;
mod favourites_and_recent_placements;
mod marker_icons_and_briefing;
mod palette_chips;
mod triggers_and_owner_links;

/// Source-inspection pins read all production modules in their render ownership order.
pub(super) fn dock_right_production_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| {
        [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/palette/mod.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/recent/mod.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/eden/mod.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/favourites/store.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/favourites/panels.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/shell/mod.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/shell/factions_panel.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/shell/layout.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/compositions/mod.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/triggers/panel.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/triggers/attributes.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/triggers/owner_line.rs"
            )),
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/frontend/workspaces/mission_creator_state/src/marker_icons.rs",
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/markers/glyph_preview.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right/markers/panel.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/docks/dock_right.rs"
            )),
        ]
        .concat()
    })
}
