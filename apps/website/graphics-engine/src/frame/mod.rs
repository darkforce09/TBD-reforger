//! Role: frame.
//! Position: `apps/website/graphics-engine/src` — the vocabulary this crate defines.
//! Signals & state: geometry and GPU handles.
//! Invariants: **these types may reference geometry and GPU handles ONLY.** A field named for
//! a thing in the world — a road, a label, a town, a coastline — is a bug in the boundary, not
//! a convenience. The caller speaks this vocabulary; this crate never speaks the caller's.

/// Frame batch.
#[cfg(target_arch = "wasm32")]
pub mod batch;

/// Frame buffers.
#[cfg(target_arch = "wasm32")]
pub mod buffers;

/// Frame camera.
pub mod camera;

/// Damage tracking — which frames need submitting at all.
pub mod damage;

/// Frame ids.
pub mod ids;

/// Frame packet.
#[cfg(target_arch = "wasm32")]
pub mod packet;

/// Frame text.
#[cfg(target_arch = "wasm32")]
pub mod text;

#[cfg(target_arch = "wasm32")]
pub use batch::{DrawBatch, DrawPayload, IndirectDraw};
#[cfg(target_arch = "wasm32")]
pub use buffers::{IndexedMesh, InstanceBuffer, VertexStream};
pub use camera::CameraUniform;
pub use ids::{BindGroupId, LaneId, PipelineId};
#[cfg(target_arch = "wasm32")]
pub use packet::FramePacket;
#[cfg(target_arch = "wasm32")]
pub use text::TextRun;
