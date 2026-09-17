//! Reexports the editor chrome surfaces and shared browser presentation helpers.

// These re-exports are all consumed through `crate::v2::apps::editor::shell::eden_chrome::*` on the wasm build; on the native
// `cargo test` shell the consumers (`editor_ops`, `select_tool`) are cfg-gated out, so allow the
// re-export shim to look unused there rather than gate each line by target.
#![allow(unused_imports)]

// The chrome insets (`STRIP_TOP_PX` / `DOCK_LEFT_PX` / `DOCK_RIGHT_PX` / `TOOLBELT_BAND_PX`) are read
// by `select_tool` and `mission_editor` to keep the input insets aligned with the panels  see
// [`crate::v2::apps::editor::shell::layout`], which owns them.
pub use crate::v2::apps::editor::shell::layout::{
    DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX, TOOLBELT_BAND_PX,
};

// The four docked components + the Mission Settings dialog `mission_editor` mounts.
pub use crate::v2::apps::editor::ui::docks::dock_left::DockLeft;
pub use crate::v2::apps::editor::ui::docks::dock_right::DockRight;
pub use crate::v2::apps::editor::ui::docks::toolbelt::BottomToolbelt;
pub use crate::v2::apps::editor::ui::docks::top_strip::TopCommandStrip;
pub use crate::v2::apps::editor::ui::modals::settings_modal::MissionSettingsDialog;

// The ORBAT Manager (near-fullscreen live graph). Implementation lives in
// [`crate::v2::apps::editor::ui::modals::orbat_manager`]; re-exported so `mission_editor`'s mount
// path stays stable.
pub use crate::v2::apps::editor::ui::modals::orbat_manager::OrbatManagerDialog;

// the zone draw tool's PURE predicates. `editor_ops` (the wasm-only doc-mutating half) calls
// these through `crate::v2::apps::editor::shell::eden_chrome`, so they stay re-exported here; they live in
// [`crate::v2::apps::editor::ui::inspector::zones_panel`].
pub use crate::v2::apps::editor::ui::inspector::zones_panel::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};
