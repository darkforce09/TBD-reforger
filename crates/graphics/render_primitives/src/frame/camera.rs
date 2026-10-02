//! Role: the 64-byte camera uniform every pipeline binds at group 0.
//! Position: `frame` in `render_primitives`; the caller composes the matrix, the graphics
//! engine's frame packet carries it.
//! Signals & state: none; one plain-old-data struct.
//! Invariants: byte-identical to the column-major matrix the caller composes.

/// Everything group 0 binding 0 carries: 64 B, column-major, WebGPU clip conventions on both
/// backends (naga performs the GL depth remap).
///
/// The caller composes the matrix — projection, zoom and the world origin it is relative to are
/// all its business. The renderer receives sixteen floats and learns nothing about the world
/// they came from.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// Clip-from-local, column-major. Composed in f64 by the caller and cast to f32.
    pub clip_from_local: [f32; 16],
}

impl CameraUniform {
    /// Wrap a already-composed column-major matrix.
    #[must_use]
    pub fn new(clip_from_local: [f32; 16]) -> Self {
        Self { clip_from_local }
    }
}

impl Default for CameraUniform {
    /// Identity — a packet built before the caller has a camera still submits.
    fn default() -> Self {
        let mut m = [0.0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        Self { clip_from_local: m }
    }
}
