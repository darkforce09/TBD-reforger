//! Role: draw compose.
//! Position: `draw` in the graphics engine.
//! Signals & state: triangulated fills and hairline segment lists, ready for upload.
//! Invariants: rings and colours in, buffers out. Which rings, and what they enclose, is
//! decided before the call — this module cannot tell a coastline from a clearing.

use crate::draw::triangulate::TriMesh;

/// Packed polygon fill for a polygon-fill lane.
#[derive(Clone, Debug, Default)]
pub struct PolyMeshGpu {
    /// Positions.
    pub positions: Vec<f32>,

    /// Colors.
    pub colors: Vec<f32>,

    /// Indices.
    pub indices: Vec<u32>,

    /// Polygon count.
    pub polygon_count: u32,
}

/// Hairline segment list for a `LineList` lane.
#[derive(Clone, Debug, Default)]
pub struct HairlineGpu {
    /// Flat `[x,y,r,g,b,a]…` — 2 verts per segment.
    pub verts: Vec<f32>,

    /// Segment count.
    pub segment_count: u32,
}

/// RGBA8 → linear 0..1, with the layer's alpha folded into the alpha channel.
#[must_use]
pub fn u8_rgba_to_f32(c: [u8; 4], layer_alpha: f32) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        (f32::from(c[3]) / 255.0) * layer_alpha,
    ]
}

/// Retint fill alpha.
pub fn retint_fill_alpha(colors: &mut [f32], alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    for c in colors.chunks_exact_mut(4) {
        c[3] = a;
    }
}

/// Mesh from tri.
#[must_use]
pub fn mesh_from_tri(mesh: TriMesh, colors_u8: &[u8], layer_alpha: f32) -> PolyMeshGpu {
    if mesh.indices.is_empty() {
        return PolyMeshGpu::default();
    }
    let n_verts = mesh.positions.len() / 2;
    let mut colors = Vec::with_capacity(n_verts * 4);
    for vi in 0..n_verts {
        let ci = vi * 4;
        let rgba = if ci + 3 < colors_u8.len() {
            [
                colors_u8[ci],
                colors_u8[ci + 1],
                colors_u8[ci + 2],
                colors_u8[ci + 3],
            ]
        } else {
            [255, 255, 255, 255]
        };
        let c = u8_rgba_to_f32(rgba, layer_alpha);
        colors.extend_from_slice(&c);
    }

    #[allow(clippy::cast_possible_truncation)]
    let polygon_count = (mesh.indices.len() / 3) as u32;
    PolyMeshGpu {
        positions: mesh.positions,
        colors,
        indices: mesh.indices,
        polygon_count,
    }
}

/// Interleaved `[x0,y0,x1,y1]…` segments → hairline verts with one fixed rgba.
#[must_use]
pub fn compose_hairlines(segments: &[f32], rgba: [u8; 4]) -> HairlineGpu {
    if segments.len() < 4 {
        return HairlineGpu::default();
    }
    let c = u8_rgba_to_f32(rgba, 1.0);
    let mut verts = Vec::with_capacity(segments.len() / 4 * 12);
    let mut segment_count = 0_u32;
    for seg in segments.chunks_exact(4) {
        for (x, y) in [(seg[0], seg[1]), (seg[2], seg[3])] {
            verts.push(x);
            verts.push(y);
            verts.extend_from_slice(&c);
        }
        segment_count += 1;
    }
    HairlineGpu {
        verts,
        segment_count,
    }
}

/// Two-colour hairlines over a set of rings: ring `i` takes `accent_rgba` when `i` is in
/// `accent_idx`, otherwise `base_rgba`.
///
/// The rings arrive as bare point slices plus a parallel `closed` flag. That flag — not the
/// caller's iso level — is the one extra column the edge loop reads: a closed ring wraps its
/// last edge back to vertex 0, an open one stops. The level decides which rings exist and
/// which are accented, both of which happen before the call.
#[must_use]
pub fn compose_two_tone_hairlines(
    rings: &[&[(f64, f64)]],
    closed: &[bool],
    accent_idx: &[usize],
    base_rgba: [u8; 4],
    accent_rgba: [u8; 4],
) -> HairlineGpu {
    let base = u8_rgba_to_f32(base_rgba, 1.0);
    let accent = u8_rgba_to_f32(accent_rgba, 1.0);
    let mut verts: Vec<f32> = Vec::new();
    let mut segment_count = 0_u32;
    for (i, points) in rings.iter().enumerate() {
        if points.len() < 2 {
            continue;
        }
        let c = if accent_idx.contains(&i) {
            &accent
        } else {
            &base
        };
        let n = points.len();

        let edges = if closed.get(i).copied().unwrap_or(false) {
            n
        } else {
            n - 1
        };
        for e in 0..edges {
            let a = points[e];
            let b = points[(e + 1) % n];
            for (x, y) in [a, b] {
                verts.push(x as f32);
                verts.push(y as f32);
                verts.extend_from_slice(c);
            }
            segment_count += 1;
        }
    }
    HairlineGpu {
        verts,
        segment_count,
    }
}
