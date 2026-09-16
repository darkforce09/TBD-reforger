//! The Mission Creator canvas nest: everything that binds the map surface to the page — the boot
//! machine, the viewport and frame-timing belt, the floating overlays, the pointer gestures and
//! the keydown dispatch, plus the tab-local hover-cursor policy.

pub mod boot;
// T-934.14 — the window-level keydown dispatch (`attach_editor_hotkeys`), riding the T-934.13
// `EditorGestureContext`. Wasm-only for the same reason as `gestures`.
#[cfg(target_arch = "wasm32")]
pub mod commands;
// T-934.13 — the pointer/wheel/dblclick/contextmenu gesture closures + `EditorGestureContext`.
// Everything inside is wasm-only (web-sys events over the live engine/doc handles), so the module
// is gated like `state/doc_host` rather than internally cfg-split.
#[cfg(target_arch = "wasm32")]
pub mod gestures;
pub mod overlays;
// The hover-cursor policy: tab-local state that dies with the tab, so it stays here while the
// picks it consults live in the map engine.
pub mod pointer_hover;
// T-936.7 — the tactical-graphics belt: ONE document read, drawn AND picked, the shape the engine's
// connection and comment lanes use. Not wasm-gated: everything but the `MissionDocCore` read is
// pure geometry, and keeping it native-testable is why the parse/pack/pick trio lives in its own
// file rather than inside the wasm-only gesture and history modules that call it.
pub mod gizmo_z;
pub mod tactical_graphics;
// The authoring half of the same lane: arm a draw, take and drop vertices, drag an authored vertex,
// delete a finished graphic. Reaches the live document, so wasm32-only — unlike the belt above it,
// which stays native-testable.
#[cfg(target_arch = "wasm32")]
pub mod tactical_graphics_authoring;
pub mod viewport;
