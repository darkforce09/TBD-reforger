//! T-934.10 — the Mission Creator canvas nest. `render_sync` (the pure helper belt split out of
//! `mission_editor.rs`) landed first; `overlays` (the floating overlay/dialog components, T-934.11)
//! followed; `boot` + `viewport` (the boot machine and the rAF/frame-timing belt, T-934.12) are
//! Phase B's third child; gestures land in T-934.13.

pub mod boot;
// T-934.13 — the pointer/wheel/dblclick/contextmenu gesture closures + `EditorGestureContext`.
// Everything inside is wasm-only (web-sys events over the live engine/doc handles), so the module
// is gated like `state/doc_host` rather than internally cfg-split.
#[cfg(target_arch = "wasm32")]
pub mod gestures;
pub mod overlays;
pub mod render_sync;
pub mod viewport;
