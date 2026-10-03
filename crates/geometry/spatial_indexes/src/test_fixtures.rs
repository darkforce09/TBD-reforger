//! Cuboid meshes for the tests of the triangle tree and of the crates that query it.
//!
//! **Role:** builds small triangle scenes: an axis-aligned cuboid ([`cube`]) and the concatenation
//! of several scenes into one mesh ([`concat`]).
//! **Position:** compiled for this crate's tests and, through the `test_fixtures` feature, for the
//! tests of the map engine's building and line-of-sight code; never in a production build.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a cuboid is 8 vertices and 12 outward-wound triangles; `concat` keeps every
//! input triangle and offsets its indices by the vertices before it.

/// A triangle mesh: vertices and index triples.
pub type Scene = (Vec<[f64; 3]>, Vec<[u32; 3]>);

/// Axis-aligned cuboid as 12 outward-wound triangles (quad table from the COLL box emitter in xtask's `xob.rs`).
pub fn cube(center: [f64; 3], half: [f64; 3]) -> Scene {
    let mut verts = Vec::new();
    for corner in 0..8u32 {
        verts.push([
            center[0] + if corner & 1 != 0 { half[0] } else { -half[0] },
            center[1] + if corner & 2 != 0 { half[1] } else { -half[1] },
            center[2] + if corner & 4 != 0 { half[2] } else { -half[2] },
        ]);
    }
    const QUADS: [[u32; 4]; 6] = [
        [0, 4, 6, 2],
        [1, 3, 7, 5],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 2, 3, 1],
        [4, 5, 7, 6],
    ];
    let mut tris = Vec::new();
    for q in QUADS {
        tris.push([q[0], q[1], q[2]]);
        tris.push([q[0], q[2], q[3]]);
    }
    (verts, tris)
}

/// One mesh holding every scene of `scenes`, in order, its indices rebased.
pub fn concat(scenes: &[Scene]) -> Scene {
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    for (v, t) in scenes {
        let base = verts.len() as u32;
        verts.extend_from_slice(v);
        tris.extend(
            t.iter()
                .map(|tri| [tri[0] + base, tri[1] + base, tri[2] + base]),
        );
    }
    (verts, tris)
}
