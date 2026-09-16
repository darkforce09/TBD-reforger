//! Role: frame text.
//! Position: `frame` in the graphics engine.
//! Signals & state: one packed glyph stream and the atlas it indexes.
//! Invariants: glyphs arrive already laid out, already placed, already sized. The renderer
//! never sees a string, a language, a place, or a reason one glyph won a contested position.

use crate::frame::buffers::InstanceBuffer;
use crate::frame::ids::{BindGroupId, LaneId, PipelineId};

/// A run of packed glyph instances.
///
/// The caller decided which glyphs exist and where they sit; this is the result of that
/// decision expressed as bytes. Each instance is the 20-byte atlas-sprite layout.
pub struct TextRun {
    /// Draw-order key.
    pub lane: LaneId,

    /// Packed glyph instances.
    pub glyphs: InstanceBuffer,

    /// The cell atlas the packed `glyph` field indexes.
    pub atlas: BindGroupId,

    /// Pipeline to bind.
    pub pipeline: PipelineId,
}
