//! T-934.10 — the Mission Creator canvas nest. `render_sync` (the pure helper belt split out of
//! `mission_editor.rs`) landed first; `overlays` (the floating overlay/dialog components, T-934.11)
//! followed; `boot` + `viewport` (the boot machine and the rAF/frame-timing belt, T-934.12) are
//! Phase B's third child; gestures land in T-934.13.

pub mod boot;
pub mod overlays;
pub mod render_sync;
pub mod viewport;
