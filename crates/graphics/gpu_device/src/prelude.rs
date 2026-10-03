//! The names most callers of the GPU device import with `use gpu_device::prelude::*;`.

pub use crate::buffers::pool::{LanePool, WriteOutcome};
pub use crate::buffers::readback::ReadbackLane;
#[cfg(target_arch = "wasm32")]
pub use crate::context::frame_acquire::Acquired;
#[cfg(target_arch = "wasm32")]
pub use crate::context::gpu_context::GpuContext;
pub use crate::context::surface_policy::BackendKind;
#[cfg(target_arch = "wasm32")]
pub use crate::context::web_display::instance_descriptor;
pub use crate::error::{Error, Result};
#[cfg(target_arch = "wasm32")]
pub use crate::timing::gpu_timer::GpuTimer;
