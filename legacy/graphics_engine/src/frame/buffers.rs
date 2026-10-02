//! Role: frame buffers.
//! Position: `frame` in the graphics engine.
//! Signals & state: GPU buffer handles plus the arithmetic needed to draw them.
//! Invariants: the renderer never interprets instance bytes — only the stride the pipeline's
//! vertex layout declares.

/// A per-instance stream drawn `draw(0..verts, 0..count)` against the unit quad at slot 0.
///
/// Today's strides are 32 B, 40 B and 20 B. The renderer does not know which is which: the
/// pipeline named by the batch declares the vertex layout, and this supplies the bytes.
pub struct InstanceBuffer {
    /// The allocation the instances live in.
    pub buffer: wgpu::Buffer,

    /// Bytes per instance.
    pub stride: u32,

    /// Instance count.
    pub count: u32,

    /// Byte offset of element 0 — pooled lanes are sub-ranges of one allocation.
    pub offset: u64,
}

impl InstanceBuffer {
    /// A whole-buffer stream starting at byte 0.
    #[must_use]
    pub fn whole(buffer: wgpu::Buffer, stride: u32, count: u32) -> Self {
        Self {
            buffer,
            stride,
            count,
            offset: 0,
        }
    }

    /// Byte span of the live instances, for `buffer.slice(..)`.
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        u64::from(self.stride) * u64::from(self.count)
    }
}

/// Indexed triangle list: interleaved vertices plus a `u32` index buffer.
pub struct IndexedMesh {
    /// Interleaved vertex stream.
    pub vertices: wgpu::Buffer,

    /// `u32` indices.
    pub indices: wgpu::Buffer,

    /// Indices to draw.
    pub index_count: u32,

    /// Source item count — carried for the caller's own statistics, never used to draw.
    pub item_count: u32,
}

/// Non-indexed vertex stream drawn as a `LineList`.
pub struct VertexStream {
    /// Interleaved vertex stream.
    pub vertices: wgpu::Buffer,

    /// Vertices to draw.
    pub vertex_count: u32,
}
