//! Role: frame.
//! Position: `apps/website/graphics-engine/src` — the vocabulary this crate defines.
//! Signals & state: geometry and GPU handles.
//! Invariants: **these types may reference geometry and GPU handles ONLY.** A field named for
//! a thing in the world — a road, a label, a town, a coastline — is a bug in the boundary, not
//! a convenience. The caller speaks this vocabulary; this crate never speaks the caller's.

/// Frame batch.
pub mod batch;

/// Frame buffers.
pub mod buffers;

/// Frame camera.
pub mod camera;

/// Frame ids.
pub mod ids;

/// Frame packet.
pub mod packet;

/// Frame text.
pub mod text;

pub use batch::{DrawBatch, DrawPayload, IndirectDraw};
pub use buffers::{IndexedMesh, InstanceBuffer, VertexStream};
pub use camera::CameraUniform;
pub use ids::{BindGroupId, LaneId, PipelineId};
pub use packet::FramePacket;
pub use text::TextRun;
