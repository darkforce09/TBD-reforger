//! `cargo xtask map voxels-from-mesh` — the mesh-based dump generator. Ray-marches real
//! triangle geometry (a Reforger `.xob`, parsed by [`super::xob`]) over the shared
//! [`super::march`] lattice and emits a standard `<slug>_voxels.jsonl.gz`, so the entire
//! offline interpreter, parity harness, and viewer run unchanged on real model data.
//!
//! Face semantics mirror the Workbench sensor: an entry recorded by the "x+" march is a
//! −X-facing surface (the face a +X ray enters). Triangles are classified by their
//! GEOMETRIC normal, oriented to agree with the mesh's packed vertex normals (robust to
//! index-winding conventions); a one-sided sheet therefore appears in exactly one march
//! direction and pair.rs absorbs it as a sliver, same as engine one-sided collision.
//! Deviations from the sensor, both parser-legal: no 48-hit cap (no trace budget here),
//! and hits closer than the engine's 0.02 m re-cast step are merged instead of re-traced.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use super::march::{self, CELL, DumpIdent, PAD};
use super::types::VoxelDump;
use super::xob;

/// Minimum separation between kept hits along one line — the sensor's STEP_PAST_M.
const MIN_SEP: f64 = 0.02;
/// |normal·axis| below this = parallel face; the engine trace does not register these.
const PARALLEL_EPS: f64 = 1e-9;

pub struct TriMesh {
    pub verts: Vec<[f64; 3]>,
    pub tris: Vec<[u32; 3]>,
    /// Per-triangle unit normal, oriented outward (matched to vertex normals).
    pub tri_normal: Vec<[f64; 3]>,
}

impl TriMesh {
    /// Build from a parsed xob: drop excluded-material triangles, apply the axes remap and
    /// optional winding flip, orient each face normal by the packed vertex normals.
    pub fn from_xob(
        m: &xob::XobMesh,
        axes: &AxesRemap,
        flip_winding: bool,
        exclude_material: &[String],
    ) -> TriMesh {
        let verts: Vec<[f64; 3]> = m.verts.iter().map(|v| axes.apply(*v)).collect();
        let vnorm: Vec<[f64; 3]> = m.vert_normals.iter().map(|v| axes.apply(*v)).collect();
        let mut tris = Vec::with_capacity(m.tris.len());
        let mut tri_normal = Vec::with_capacity(m.tris.len());
        let mut dropped = 0usize;
        for (t, tri) in m.tris.iter().enumerate() {
            if !exclude_material.is_empty() {
                let sm = m.tri_submesh.get(t).copied().unwrap_or(0) as usize;
                if let Some(mat) = m.materials.get(sm)
                    && exclude_material.iter().any(|x| mat.contains(x.as_str()))
                {
                    dropped += 1;
                    continue;
                }
            }
            let [a, b, c] = *tri;
            let (va, vb, vc) = (verts[a as usize], verts[b as usize], verts[c as usize]);
            let g = cross(sub(vb, va), sub(vc, va));
            let len = dot(g, g).sqrt();
            if len < 1e-12 {
                continue; // degenerate
            }
            let mut n = [g[0] / len, g[1] / len, g[2] / len];
            let avg = [
                vnorm[a as usize][0] + vnorm[b as usize][0] + vnorm[c as usize][0],
                vnorm[a as usize][1] + vnorm[b as usize][1] + vnorm[c as usize][1],
                vnorm[a as usize][2] + vnorm[b as usize][2] + vnorm[c as usize][2],
            ];
            if dot(n, avg) < 0.0 {
                n = [-n[0], -n[1], -n[2]];
            }
            if flip_winding {
                n = [-n[0], -n[1], -n[2]];
            }
            tris.push(*tri);
            tri_normal.push(n);
        }
        if dropped > 0 {
            println!("  excluded {dropped} triangles by material filter");
        }
        TriMesh {
            verts,
            tris,
            tri_normal,
        }
    }
}

/// Axis remap like "x,y,-z": output axis i takes input axis `perm[i]` times `sign[i]`.
pub struct AxesRemap {
    perm: [usize; 3],
    sign: [f64; 3],
}

impl AxesRemap {
    pub fn identity() -> AxesRemap {
        AxesRemap {
            perm: [0, 1, 2],
            sign: [1.0, 1.0, 1.0],
        }
    }

    pub fn parse(spec: &str) -> Result<AxesRemap> {
        let mut perm = [0usize; 3];
        let mut sign = [1.0f64; 3];
        let parts: Vec<&str> = spec.split(',').collect();
        if parts.len() != 3 {
            bail!("--axes wants three comma-separated tokens, e.g. x,y,-z");
        }
        for (i, raw) in parts.iter().enumerate() {
            let t = raw.trim();
            let (s, name) = match t.strip_prefix('-') {
                Some(rest) => (-1.0, rest),
                None => (1.0, t),
            };
            perm[i] = match name {
                "x" => 0,
                "y" => 1,
                "z" => 2,
                other => bail!("--axes token '{other}' is not x|y|z"),
            };
            sign[i] = s;
        }
        Ok(AxesRemap { perm, sign })
    }

    fn apply(&self, v: [f64; 3]) -> [f64; 3] {
        [
            self.sign[0] * v[self.perm[0]],
            self.sign[1] * v[self.perm[1]],
            self.sign[2] * v[self.perm[2]],
        ]
    }
}

/// Per-axis triangle bins over the cross-axes lattice: bin index == scanline (j, k).
struct AxisBins {
    c1: usize,
    c2: usize,
    d1: usize,
    d2: usize,
    origin: [f64; 3],
    bins: Vec<Vec<u32>>,
}

impl AxisBins {
    fn build(mesh: &TriMesh, axis: usize, origin: [f64; 3], dims: [usize; 3]) -> AxisBins {
        let (c1, c2) = cross_axes(axis);
        let (d1, d2) = (dims[c1], dims[c2]);
        let mut bins = vec![Vec::new(); d1 * d2];
        let cell_range = |lo: f64, hi: f64, c: usize, d: usize| -> (usize, usize) {
            // Lines sit at origin[c] + (i + 0.5)·CELL; cover every line inside [lo, hi]
            // with one cell of slack for the exact-boundary case.
            let a = ((lo - origin[c]) / CELL - 0.5).floor() as i64 - 1;
            let b = ((hi - origin[c]) / CELL - 0.5).ceil() as i64 + 1;
            (
                a.clamp(0, d as i64 - 1) as usize,
                b.clamp(0, d as i64 - 1) as usize,
            )
        };
        for (t, tri) in mesh.tris.iter().enumerate() {
            let mut lo = [f64::MAX; 3];
            let mut hi = [f64::MIN; 3];
            for &vi in tri {
                let v = mesh.verts[vi as usize];
                for a in 0..3 {
                    lo[a] = lo[a].min(v[a]);
                    hi[a] = hi[a].max(v[a]);
                }
            }
            let (j0, j1) = cell_range(lo[c1], hi[c1], c1, d1);
            let (k0, k1) = cell_range(lo[c2], hi[c2], c2, d2);
            for j in j0..=j1 {
                for k in k0..=k1 {
                    bins[j * d2 + k].push(t as u32);
                }
            }
        }
        AxisBins {
            c1,
            c2,
            d1,
            d2,
            origin,
            bins,
        }
    }

    fn candidates(&self, a: f64, b: f64) -> &[u32] {
        let j = ((a - self.origin[self.c1]) / CELL - 0.5).round() as i64;
        let k = ((b - self.origin[self.c2]) / CELL - 0.5).round() as i64;
        if j < 0 || k < 0 || j as usize >= self.d1 || k as usize >= self.d2 {
            return &[];
        }
        &self.bins[j as usize * self.d2 + k as usize]
    }
}

#[cfg(test)]
#[path = "../tests/mesh/tests.rs"]
mod tests;

#[path = "mesh_voxelization/sub.rs"]
mod sub;
use sub::backface_first_fraction;
use sub::cross;
use sub::cross_axes;
use sub::dot;
pub use sub::generate;
use sub::print_stats;
use sub::sub;
pub use sub::write_dump;

#[path = "mesh_voxelization/run_voxels_from_mesh.rs"]
mod run_voxels_from_mesh;
pub use run_voxels_from_mesh::run_voxels_from_mesh;

#[cfg(test)]
pub(crate) use sub::min_sep;
