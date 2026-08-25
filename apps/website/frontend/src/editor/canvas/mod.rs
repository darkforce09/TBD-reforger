//! T-934.10 — the Mission Creator canvas nest. `render_sync` (the pure helper belt split out of
//! `mission_editor.rs`) lands first; overlays, boot, viewport and gestures follow in the later
//! Phase B children (T-934.11–.13).

pub mod render_sync;
