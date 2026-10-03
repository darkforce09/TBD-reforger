//! Reexports the editor chrome surfaces and shared browser presentation helpers.

// Each re-export carries the gate of its consumers: the mounted components exist on the wasm32
// build only. The chrome insets are read straight from
// [`crate::workspaces::editor::session::layout`], which owns them.

// The four docked components + the Mission Settings dialog `mission_editor` mounts.
#[cfg(target_arch = "wasm32")]
pub use crate::workspaces::editor::ui::docks::dock_left::DockLeft;
#[cfg(target_arch = "wasm32")]
pub use crate::workspaces::editor::ui::docks::dock_right::DockRight;
#[cfg(target_arch = "wasm32")]
pub use crate::workspaces::editor::ui::docks::top_strip::TopCommandStrip;
#[cfg(target_arch = "wasm32")]
pub use crate::workspaces::editor::ui::modals::settings_modal::MissionSettingsDialog;

// The ORBAT Manager (near-fullscreen live graph). Implementation lives in
// [`crate::workspaces::editor::ui::modals::orbat_manager`]; re-exported so `mission_editor`'s mount
// path stays stable.
#[cfg(target_arch = "wasm32")]
pub use crate::workspaces::editor::ui::modals::orbat_manager::OrbatManagerDialog;

// the zone draw tool's PURE predicates. `editor_ops` (the wasm-only doc-mutating half) calls
// these through `crate::workspaces::editor::session::eden_chrome`, so they stay re-exported here; they live in
// [`crate::workspaces::editor::ui::inspector::zones_panel`].
#[cfg(target_arch = "wasm32")]
pub use crate::workspaces::editor::ui::inspector::zones_panel::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};
