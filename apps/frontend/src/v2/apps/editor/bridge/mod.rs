//! The Mission Creator's side of the engine seam: everything that drives its frames.
//!
//! **Role:** owns the machinery that keeps the editor's map current — the boot machine that
//! raises the document and the world, the frame-loop readouts on the shared map seam's pump
//! (`crate::v2::core::map_view`, which also sizes the canvas), the floating overlays laid over
//! the map, the pure geometry the renderer and the pick paths share, the asset host that feeds
//! terrain and imagery in, the hosted document with its undo drive, and the host signal state the
//! engine's hosted commands read.
//! **Position:** between the canvas mount (`mission_editor/canvas_mount.rs`, which creates the
//! engine, document and host handles) and `map_engine`; the document commands
//! (`shell/document_commands.rs`) and the pointer gestures (`input/pointer_gestures.rs`) hold
//! those handles too. The docked chrome under [`super::ui`] and the interactive tools under
//! [`super::input::tools`] reach the map through [`host_state`] and the command layers, never
//! through a handle of their own.
//! **Signals & state:** the boot phase, the frame-timing samples, the widget-pivot registry and
//! the hover cursor are all tab-local — they die with the browser tab and never reach the
//! document. Anything an operator authored travels through `map_engine::editing` instead.
//! **Invariants:** a module that touches `web_sys` or a live engine handle is
//! `#[cfg(target_arch = "wasm32")]` and its `pub mod` line carries the same gate, so the native
//! test build still compiles the pure half of the seam. What the canvas draws and what a click can
//! pick are produced by one read of the document, never by two parsers kept in step by hand.

/// The boot machine: the phases a mounting editor passes through, the per-segment progress
/// arithmetic behind the boot overlay, and the hand-over that hides it once the world settles.
pub mod boot;
/// The hosted mission document and the undo drive that moves it: the document's lifecycle and
/// smoke bridge, and the single driver the toolbar, the shortcuts and the harness all take.
pub mod document_host;
/// The transform gizmo's vertical Z arm: its geometry, its hit test, and the pure arithmetic that
/// turns a vertical drag into snapped metres of elevation.
pub mod gizmo_z;
/// The host signal state the engine's hosted commands read: the installed editor context, the
/// in-flight placement, the selected entities and the host half of undo grouping.
pub mod host_state;
/// The floating overlays and dialogs laid over the map: the transform widget and its mode hint and
/// snap readout, the empty-ground asset picker, the comment editor, the connections panel and the
/// local-versus-server conflict dialog, plus the widget-pivot registry the gizmo reads.
pub mod overlays;
/// The hover-cursor policy: the throttled, transition-driven state machine that decides whether
/// the pixel under the pointer is pickable and what cursor the canvas therefore wears.
pub mod pointer_hover;
/// The tactical-graphics belt: one document read, drawn and picked. Parses the authored control
/// measures, packs them for the renderer, and hit-tests the same set a click resolves against.
pub mod tactical_graphics;
/// The authoring half of the tactical-graphics lane: arm a multi-click draw, take and drop
/// vertices, drag an authored vertex, and delete a finished graphic. Reaches the live document.
#[cfg(target_arch = "wasm32")]
pub mod tactical_graphics_authoring;
/// The frame-timing belt: the editor's hook on the shared frame pump with its debug-HUD sample
/// and scale publish, the window-gate registrars the headless harness
/// drives, and the registry-fetch failure signal and its session cache.
pub mod viewport;
/// The map-asset host: supplies the editor's live layer, basemap and render preferences to the
/// engine's streaming host and registers the mounted engine and host pair with owner cleanup.
/// All asset fetching, residency, geometry and upload work lives in the engines themselves.
#[cfg(target_arch = "wasm32")]
pub mod world_assets;
