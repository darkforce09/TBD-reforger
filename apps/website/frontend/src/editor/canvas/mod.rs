//! T-934.10 — the Mission Creator canvas nest. `render_sync` (the pure helper belt split out of
//! `mission_editor.rs`) landed first; `overlays` (the floating overlay/dialog components, T-934.11)
//! followed; boot, viewport and gestures land in the later Phase B children (T-934.12–.13).

pub mod overlays;
pub mod render_sync;
