//! The names most users of the map renderer import with `use map_renderer::prelude::*;`.

#[cfg(target_arch = "wasm32")]
pub use crate::engine::{EngineHandle, RenderEngine};
pub use crate::error::{Error, Result};
