//! The shared map seam: everything a page needs to put a live, navigable terrain map on a
//! canvas.
//!
//! **Role:** canvas sizing, render engine creation, the camera fitted to a terrain manifest's
//! `worldBounds`, the damage-driven frame pump, resize tracking, drag-pan, wheel-zoom and
//! click-to-pick in map metres, and 2 m ground heights from the full-resolution elevation raster.
//! **Position:** a foundation crate between the pages and the map crates (`map_renderer`,
//! `map_streaming_host`). The Mission Creator builds its canvas mount from these parts and keeps
//! its tools to itself; map pickers call `mount::mount_map_view` with [`terrain_preferences`].
//! **Signals & state:** per mount, the shared slots of `handles::MapViewHandles`; no
//! module-level state.
//! **Invariants:** depends on no frontend crate; the pure half (sizing, camera fit, navigation
//! arithmetic, heights, preferences) and the error compile on every target and are unit-tested
//! natively; the browser half is `#[cfg(target_arch = "wasm32")]` on its `pub mod` line, since it
//! calls the DOM and the wasm32-only map crates.

pub mod camera_fit;
pub mod device_size;
pub mod error;
pub mod navigation_math;
pub mod prelude;
pub mod terrain_height;
pub mod terrain_preferences;

#[cfg(target_arch = "wasm32")]
pub mod engine_mount;
#[cfg(target_arch = "wasm32")]
pub mod frame_pump;
#[cfg(target_arch = "wasm32")]
pub mod handles;
#[cfg(target_arch = "wasm32")]
pub mod mount;
#[cfg(target_arch = "wasm32")]
pub mod navigation;
#[cfg(target_arch = "wasm32")]
pub mod resize;

pub use error::{Error, Result};
