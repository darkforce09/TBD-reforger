//! T-159.21 — Eden chrome for the Mission Creator (/missions/:id/edit).
//!
//! T-661 split this 5,119-line module by symbol into ten `eden_*` siblings (before the editor
//! program starts). This file is now a thin re-export shim so consumers (`mission_editor`,
//! `select_tool`, `editor_ops`) keep importing from `crate::eden_chrome::*` unchanged — the split
//! is a PURE MOVE, no behaviour change.
//!
//! The docked shell React renders around the map: a Top Command Strip (title, Undo/Redo, the
//! T-159.20 Save/Export controls, Settings), a Bottom Toolbelt (Select + CUR/SEL/OBJ readout), and
//! left/right docks (the Editor Layers outliner and the Factions/Vehicles/Zones palette). The
//! per-module docs carry the detail; the layering contract lives with the constants in
//! [`crate::eden_layout`].
//!
//! **Layering (React MissionCreatorPage:272):** the chrome overlays a full-bleed canvas; it never
//! shrinks it. Every `select_tool` probe builds its camera from the container's bounding rect, so a
//! resized container would silently invalidate the pan/select/marquee/move gates. The panels are
//! absolutely positioned inside the gesture container instead, and the host div stops `pointerdown`
//! from bubbling into the map handlers (see `mission_editor`'s view).

// These re-exports are all consumed through `crate::eden_chrome::*` on the wasm build; on the native
// `cargo test` shell the consumers (`editor_ops`, `select_tool`) are cfg-gated out, so allow the
// re-export shim to look unused there rather than gate each line by target.
#![allow(unused_imports)]

// The chrome insets (`STRIP_TOP_PX` / `DOCK_LEFT_PX` / `DOCK_RIGHT_PX` / `TOOLBELT_BAND_PX`) are read
// by `select_tool` and `mission_editor` to keep the input insets aligned with the panels — see
// [`crate::eden_layout`], which owns them.
pub use crate::eden_layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX, TOOLBELT_BAND_PX};

// The four docked components + the Mission Settings dialog `mission_editor` mounts.
pub use crate::eden_dock_left::DockLeft;
pub use crate::eden_dock_right::DockRight;
pub use crate::eden_settings::MissionSettingsDialog;
pub use crate::eden_toolbelt::BottomToolbelt;
pub use crate::eden_top_strip::TopCommandStrip;

// T-180.7 — Stitch ORBAT Manager (near-fullscreen live graph). Implementation lives in
// [`crate::pages::operations::orbat_manager`]; re-exported so `mission_editor`'s mount path stays stable.
pub use crate::pages::operations::orbat_manager::OrbatManagerDialog;

// T-582 — the zone draw tool's PURE predicates. `editor_ops` (the wasm-only doc-mutating half) calls
// these through `crate::eden_chrome`, so they stay re-exported here; they live in
// [`crate::eden_zones`].
pub use crate::eden_zones::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};
