//! The names a frontend imports with `use map_streaming_host::prelude::*;`.

#[cfg(target_arch = "wasm32")]
pub use crate::{
    DemGridHandle, HostHandle, MapHost, RENDER_CTX, bootstrap, flush_viewport, new_dem_grid_handle,
    new_host_handle, schedule_camera_settle, set_camera_gesture,
};
