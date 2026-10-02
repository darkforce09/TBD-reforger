//! The names most callers of the render primitives import with
//! `use render_primitives::prelude::*;`.

pub use crate::color_normalization::{norm, u8_rgba_to_f32};
pub use crate::draw::compose::{HairlineGpu, PolyMeshGpu};
pub use crate::draw::geometry::LineVertex;
pub use crate::draw::instances::{BuildingInstance, IconInstance, QuadInstance};
pub use crate::draw::triangulate::TriMesh;
pub use crate::frame::camera::CameraUniform;
pub use crate::frame::damage::{FrameDecision, RenderDamage};
pub use crate::frame::ids::{BindGroupId, LaneId, PipelineId};
pub use crate::text::layout::{GlyphSpec, GlyphSpecId};
pub use crate::text::metrics::TextGlyphInstance;
