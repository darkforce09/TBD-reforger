//! Role: draw lines.
//! Position: `draw` in the graphics engine.
//! Signals & state: `LineList` vertex streams and the buffers they live in.
//! Invariants: a flat array of coordinates and colours in, a GPU buffer out. Whether the
//! segments are a grid, a boundary or a link between two things is never asked.

use crate::draw::geometry::{LineVertex, rel};
use crate::frame::buffers::VertexStream;

/// Upload already-built vertices as a `LineList` stream.
#[must_use]
pub fn upload_line_stream(
    device: &wgpu::Device,
    label: &str,
    verts: &[LineVertex],
) -> VertexStream {
    use wgpu::util::DeviceExt;
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    #[allow(clippy::cast_possible_truncation)]
    let vertex_count = verts.len() as u32;
    VertexStream {
        vertices,
        vertex_count,
    }
}

/// Stride-6 `[x, y, r, g, b, a]…` in world meters → an anchor-relative `LineList` stream.
///
/// The caller checks the length and decides what an empty or ragged array means; by the time
/// this runs the array is known good.
#[must_use]
pub fn upload_hairlines(
    device: &wgpu::Device,
    label: &str,
    anchor: [f64; 2],
    packed: &[f32],
) -> VertexStream {
    const STRIDE: usize = 6;
    let mut verts = Vec::with_capacity(packed.len() / STRIDE);
    for c in packed.chunks_exact(STRIDE) {
        verts.push(LineVertex {
            pos: rel(anchor, f64::from(c[0]), f64::from(c[1])),
            color: [c[2], c[3], c[4], c[5]],
        });
    }
    upload_line_stream(device, label, &verts)
}
