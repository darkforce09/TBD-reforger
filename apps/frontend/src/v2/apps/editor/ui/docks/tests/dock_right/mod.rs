use super::*;
use leptos::prelude::*;

mod compositions;
mod favourites_and_recent_placements;
mod marker_icons_and_briefing;
mod palette_chips;
mod triggers_and_owner_links;

/// Source-inspection pins read all production modules in their render ownership order.
pub(super) const DOCK_RIGHT_PRODUCTION_SOURCE: &str = concat!(
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/palette/mod.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/recent/mod.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/eden/mod.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/favourites/store.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/favourites/panels.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/shell/mod.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/shell/factions_panel.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/shell/layout.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/compositions/mod.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/triggers/panel.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/triggers/attributes.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/triggers/owner_line.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/markers/icons.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right/markers/panel.rs"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    )),
);
