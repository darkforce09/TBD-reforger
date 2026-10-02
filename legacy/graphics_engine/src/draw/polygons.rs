//! Role: draw polygons.
//! Position: `draw` in the graphics engine.
//! Signals & state: indexed triangle meshes and the buffers they live in.
//! Invariants: positions, colours and indices in, a GPU mesh out. `item_count` is carried
//! through untouched — it is the caller's own statistic, never used to draw.

use crate::draw::geometry::{LineVertex, rel};
use crate::frame::buffers::IndexedMesh;

/// Upload already-built vertices and indices as an indexed triangle list.
#[must_use]
pub fn upload_indexed_mesh(
    device: &wgpu::Device,
    vertex_label: &str,
    index_label: &str,
    verts: &[LineVertex],
    indices: &[u32],
    item_count: u32,
) -> IndexedMesh {
    use wgpu::util::DeviceExt;
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(vertex_label),
        contents: bytemuck::cast_slice(verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(index_label),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    #[allow(clippy::cast_possible_truncation)]
    let index_count = indices.len() as u32;
    IndexedMesh {
        vertices,
        indices: index_buf,
        index_count,
        item_count,
    }
}

/// Parallel `positions` (`[x, y]…`, world meters) + `colors` (`[r, g, b, a]…`) + `indices`
/// → an anchor-relative indexed mesh.
#[must_use]
pub fn upload_polygon_mesh(
    device: &wgpu::Device,
    anchor: [f64; 2],
    positions: &[f32],
    colors: &[f32],
    indices: &[u32],
    item_count: u32,
) -> IndexedMesh {
    let n_verts = positions.len() / 2;
    let mut verts = Vec::with_capacity(n_verts);
    for i in 0..n_verts {
        verts.push(LineVertex {
            pos: rel(
                anchor,
                f64::from(positions[i * 2]),
                f64::from(positions[i * 2 + 1]),
            ),
            color: [
                colors[i * 4],
                colors[i * 4 + 1],
                colors[i * 4 + 2],
                colors[i * 4 + 3],
            ],
        });
    }
    upload_indexed_mesh(
        device,
        "polygon-verts",
        "polygon-indices",
        &verts,
        indices,
        item_count,
    )
}

/// Stride-6 `[x, y, r, g, b, a]…` triangle-soup vertices → an anchor-relative indexed mesh
/// with the trivial `0..n` index buffer.
#[must_use]
pub fn upload_strip_mesh(
    device: &wgpu::Device,
    anchor: [f64; 2],
    packed: &[f32],
    item_count: u32,
) -> IndexedMesh {
    const STRIDE: usize = 6;
    let n_verts = packed.len() / STRIDE;
    let mut verts = Vec::with_capacity(n_verts);
    for c in packed.chunks_exact(STRIDE) {
        verts.push(LineVertex {
            pos: rel(anchor, f64::from(c[0]), f64::from(c[1])),
            color: [c[2], c[3], c[4], c[5]],
        });
    }

    #[allow(clippy::cast_possible_truncation)]
    let indices: Vec<u32> = (0..n_verts as u32).collect();
    upload_indexed_mesh(
        device,
        "strip-verts",
        "strip-indices",
        &verts,
        &indices,
        item_count,
    )
}
