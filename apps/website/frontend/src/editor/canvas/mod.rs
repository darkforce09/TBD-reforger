//! T-934.10 — the Mission Creator canvas nest. `render_sync` (the pure helper belt split out of
//! `mission_editor.rs`) landed first; `overlays` (the floating overlay/dialog components, T-934.11)
//! followed; `boot` + `viewport` (the boot machine and the rAF/frame-timing belt, T-934.12) are
//! Phase B's third child; gestures land in T-934.13.

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
pub mod render_sync;
// T-936.7 — the tactical-graphics belt: ONE document read, drawn AND picked (`render_sync`'s
// T-780/T-784 shape). Not wasm-gated: everything but the `MissionDocCore` read is pure geometry,
// and keeping it native-testable is why the parse/pack/pick trio lives in its own file rather than
// inside the wasm-only gesture and history modules that call it.
pub mod tactical_graphics;
pub mod viewport;
