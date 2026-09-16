//! The Mission Creator canvas input nest: the pointer gestures over the map surface and the
//! window-level keydown dispatch that shares their context.

// T-934.14 — the window-level keydown dispatch (`attach_editor_hotkeys`), riding the T-934.13
// `EditorGestureContext`. Wasm-only for the same reason as `gestures`.
#[cfg(target_arch = "wasm32")]
pub mod commands;
// T-934.13 — the pointer/wheel/dblclick/contextmenu gesture closures + `EditorGestureContext`.
// Everything inside is wasm-only (web-sys events over the live engine/doc handles), so the module
// is gated like `state/doc_host` rather than internally cfg-split.
#[cfg(target_arch = "wasm32")]
pub mod gestures;
