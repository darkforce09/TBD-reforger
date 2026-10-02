//! Role: Module boundary for the selection tool.
//! Position: `editing/tools` in the map engine.
//! Signals & state: a frozen camera snapshot and the app-side selected-id set.
//! Invariants: selection is app state, never document state — a mission is the same mission
//! whatever is highlighted. Every pick and marquee runs against the camera frozen at press time.

/// The left-button gesture model.
pub mod gesture;

/// Rectangle selection and the in-view census.
pub mod marquee;

/// Freeze a camera, resolve a screen point to an entity.
pub mod pick;

/// The indexed pick and marquee checked against a brute-force oracle.
pub mod self_check;

#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use gesture::EngineHandle;
/// The tool's vocabulary, flat: a consumer names `selection::frozen_camera`, not the file it
/// happens to sit in.
pub use gesture::{
    DRAG_THRESHOLD_PX, LeftGesture, PendingLeft, SelectionHandle, may_promote_pending,
};
pub use marquee::{
    marquee_ids, marquee_ids_with_vehicles, marquee_vehicle_ids, view_ids_with_vehicles,
};
pub use pick::{
    apply_click, compute_move_ids, drag_delta, frozen_camera, pick, pick_slot_or_vehicle,
    pick_vehicle,
};
pub use self_check::{marquee_selfcheck, pick_selfcheck};
