//! The frontend's side of the engine seam: the canvas mount and everything that drives a frame.
//!
//! **Role:** owns the surface the map is drawn on and the machinery that keeps it current — the
//! boot machine that raises the document and the world, the viewport belt that sizes the backing
//! store and paces the frame pump, the floating overlays laid over the map, the pure geometry the
//! renderer and the pick paths share, and the asset host that feeds terrain and imagery in.
//! **Position:** the only place in the editor that holds a live engine or host handle and hands it
//! to `website_map_engine`. The docked chrome under [`super::panels`] and the interactive tools
//! under [`super::input::tools`] reach the map through the state and command layers, never through
//! a handle of their own.
//! **Signals & state:** the boot phase, the frame-timing samples, the widget-pivot registry and
//! the hover cursor are all tab-local — they die with the browser tab and never reach the
//! document. Anything an operator authored travels through `website_map_engine::editing` instead.
//! **Invariants:** a module that touches `web_sys` or a live engine handle is
//! `#[cfg(target_arch = "wasm32")]` and its `pub mod` line carries the same gate, so the native
//! test build still compiles the pure half of the seam. What the canvas draws and what a click can
//! pick are produced by one read of the document, never by two parsers kept in step by hand.

/// The boot machine: the phases a mounting editor passes through, the per-segment progress
/// arithmetic behind the boot overlay, and the hand-over that hides it once the world settles.
pub mod boot;
/// The transform gizmo's vertical Z arm: its geometry, its hit test, and the pure arithmetic that
/// turns a vertical drag into snapped metres of elevation.
pub mod gizmo_z;
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
/// The viewport and frame-timing belt: CSS-to-device-pixel sizing, the frame pump's leptos half
/// with its debug-HUD sample and scale publish, the window-gate registrars the headless harness
/// drives, and the registry-fetch failure signal and its session cache.
pub mod viewport;
/// The map-asset host: supplies the editor's live layer, basemap and render preferences to the
/// engine's streaming host and registers the mounted engine and host pair with owner cleanup.
/// All asset fetching, residency, geometry and upload work lives in the engines themselves.
#[cfg(target_arch = "wasm32")]
pub mod world_assets;
