//! Role: frame.
//! Position: `legacy/graphics_engine/src` — the vocabulary this crate defines.
//! Signals & state: geometry and GPU handles; the GPU-free ids, damage tracking and camera
//! uniform live in `render_primitives::frame`.
//! Invariants: **these types may reference geometry and GPU handles ONLY.** A field named for
//! a thing in the world — a road, a label, a town, a coastline — is a bug in the boundary, not
//! a convenience. The caller speaks this vocabulary; this crate never speaks the caller's.

/// Cell atlases — the textures, uniform buffers and bind groups a `TextRun` indexes.
#[cfg(target_arch = "wasm32")]
pub mod atlas;

/// Frame batch.
#[cfg(target_arch = "wasm32")]
pub mod batch;

/// Frame buffers.
#[cfg(target_arch = "wasm32")]
pub mod buffers;

/// Frame packet.
#[cfg(target_arch = "wasm32")]
pub mod packet;

/// Swapchain acquire, submit and present.
#[cfg(target_arch = "wasm32")]
pub mod present;

/// Frame text.
#[cfg(target_arch = "wasm32")]
pub mod text;

#[cfg(target_arch = "wasm32")]
pub use atlas::{GlyphAtlasGpu, TextAtlasGpu, create_glyph_atlas, create_text_atlas};
#[cfg(target_arch = "wasm32")]
pub use batch::{DrawBatch, DrawPayload, IndirectDraw};
#[cfg(target_arch = "wasm32")]
pub use buffers::{IndexedMesh, InstanceBuffer, VertexStream};
#[cfg(target_arch = "wasm32")]
pub use packet::FramePacket;
#[cfg(target_arch = "wasm32")]
pub use text::TextRun;
