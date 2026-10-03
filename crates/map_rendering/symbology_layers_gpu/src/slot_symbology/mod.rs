//! **Role:** the slot symbology typed layer: [`SlotSymbologyGpu`], the owned GPU state of the
//! slot atlas, slot bridge and pooled sprite lanes, and [`SlotSymbology`], that state at work
//! with the renderer parts its binds write through.
//! **Position:** `symbology_layers_gpu::slot_symbology`; the renderer holds a
//! [`SlotSymbologyGpu`] as a field and lends it out as a [`SlotSymbology`] to the Mission
//! Creator's calls and to its own camera hook.
//! **Signals & state:** the slot atlas, the slot bridge columns, selection and drag, the pooled
//! lane buffers; owned by `state`.
//! **Invariants:** every lane write goes through the renderer's lane sink; the files split the
//! binds by concern and share the helpers in `view`.

/// The slot atlas upload and the zoom uniform sync.
mod atlas;

/// The camera change and the cluster marker lane.
mod clusters;

/// The slot drag overlay.
mod drag;

/// The vehicle, marker, comment and placement preview lanes.
mod mission_lanes;

/// The pooled sprite lane uploads.
mod pooled_lanes;

/// The slot lane binds, repacks and selection patches.
mod slot_lane;

/// The owned GPU state.
mod state;

/// The borrowed working view and its shared helpers.
mod view;

pub use pooled_lanes::is_pooled_icon_lane;
pub use state::SlotSymbologyGpu;
pub use view::{SlotSymbology, TextAtlasSupply};
