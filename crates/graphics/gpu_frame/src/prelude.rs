//! The names most callers of the GPU frame import with `use gpu_frame::prelude::*;`: the frame
//! vocabulary and the pump.

pub use crate::error::{Error, Result};
#[cfg(target_arch = "wasm32")]
pub use crate::frame::{
    DrawBatch, DrawPayload, FramePacket, GlyphAtlasGpu, IndexedMesh, IndirectDraw, InstanceBuffer,
    TextAtlasGpu, TextRun, VertexStream,
};
pub use crate::frame_pump::{FrameTarget, RafPump};
