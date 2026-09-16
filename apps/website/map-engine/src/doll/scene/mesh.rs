//! Role: mesh.
//! Position: `doll/scene` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Unit cube (extent ±0.5), per-face normals: 24 vertices × 6 floats (pos+normal interleaved), 36 indices.
#[must_use]
pub fn mesh_cube() -> (Vec<f32>, Vec<u16>) {
    const FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [0.0, 0.0, 1.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
        ),
        (
            [1.0, 0.0, 0.0],
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
        ),
        (
            [0.0, 1.0, 0.0],
            [
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
        ),
    ];
    let mut verts = Vec::with_capacity(24 * 6);
    let mut idx = Vec::with_capacity(36);
    for (f, (n, corners)) in FACES.iter().enumerate() {
        let base = u16::try_from(f * 4).expect("cube base fits u16");
        for c in corners {
            verts.extend_from_slice(c);
            verts.extend_from_slice(n);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

/// Unit cylinder (axis Y, radius 0.5, height 1): `segments` side quads with radial normals plus two cap fans with axial normals. Vertices: `4·s` side + `2·(s+1)` caps; indices: `6·s` side + `2·3·s` caps.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn mesh_cylinder(segments: usize) -> (Vec<f32>, Vec<u16>) {
    let s = segments.max(3);
    let mut verts: Vec<f32> = Vec::new();
    let mut idx: Vec<u16> = Vec::new();
    let ring = |i: usize| {
        let a = (i as f64) / (s as f64) * core::f64::consts::TAU;
        (0.5 * a.cos(), 0.5 * a.sin())
    };

    for i in 0..s {
        let (x0, z0) = ring(i);
        let (x1, z1) = ring(i + 1);
        let n = {
            let (nx, nz) = ((x0 + x1) as f32, (z0 + z1) as f32);
            let len = (nx * nx + nz * nz).sqrt().max(1e-6);
            [nx / len, 0.0, nz / len]
        };
        let base = u16::try_from(verts.len() / 6).expect("cylinder verts fit u16");
        for (x, y, z) in [(x0, -0.5, z0), (x1, -0.5, z1), (x1, 0.5, z1), (x0, 0.5, z0)] {
            verts.extend_from_slice(&[x as f32, y as f32, z as f32]);
            verts.extend_from_slice(&n);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    for (y, ny) in [(0.5_f64, 1.0_f32), (-0.5, -1.0)] {
        let center = u16::try_from(verts.len() / 6).expect("cap center fits u16");
        verts.extend_from_slice(&[0.0, y as f32, 0.0, 0.0, ny, 0.0]);
        for i in 0..s {
            let (x, z) = ring(i);
            verts.extend_from_slice(&[x as f32, y as f32, z as f32, 0.0, ny, 0.0]);
        }
        for i in 0..s {
            let a = center + 1 + u16::try_from(i).expect("cap idx");
            let b = center + 1 + u16::try_from((i + 1) % s).expect("cap idx");
            if ny > 0.0 {
                idx.extend_from_slice(&[center, a, b]);
            } else {
                idx.extend_from_slice(&[center, b, a]);
            }
        }
    }
    (verts, idx)
}
