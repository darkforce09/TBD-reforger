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

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
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
                if let Some(mat) = m.materials.get(sm) {
                    if exclude_material.iter().any(|x| mat.contains(x.as_str())) {
                        dropped += 1;
                        continue;
                    }
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

fn cross_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
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

/// Möller–Trumbore against an axis-aligned line at cross coords (a, b); returns the axis
/// coordinate of the hit, or None.
fn line_tri_hit(mesh: &TriMesh, t: usize, axis: usize, a: f64, b: f64) -> Option<f64> {
    let (c1, c2) = cross_axes(axis);
    let [ia, ib, ic] = mesh.tris[t];
    let v0 = mesh.verts[ia as usize];
    let v1 = mesh.verts[ib as usize];
    let v2 = mesh.verts[ic as usize];
    let mut o = [0.0f64; 3];
    o[c1] = a;
    o[c2] = b;
    let mut dir = [0.0f64; 3];
    dir[axis] = 1.0;
    let e1 = sub(v1, v0);
    let e2 = sub(v2, v0);
    let pvec = cross(dir, e2);
    let det = dot(e1, pvec);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = sub(o, v0);
    let u = dot(tvec, pvec) * inv;
    if !(-1e-9..=1.0 + 1e-9).contains(&u) {
        return None;
    }
    let qvec = cross(tvec, e1);
    let v = dot(dir, qvec) * inv;
    if v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    Some(dot(e2, qvec) * inv)
}

/// All entry faces along one line, split by orientation: `fwd` = faces entered marching
/// +axis (normal·axis < 0), ascending; `bwd` = faces entered marching −axis, descending.
fn line_hits(mesh: &TriMesh, bins: &AxisBins, axis: usize, a: f64, b: f64) -> (Vec<f64>, Vec<f64>) {
    let mut fwd = Vec::new();
    let mut bwd = Vec::new();
    for &t in bins.candidates(a, b) {
        let n_axis = mesh.tri_normal[t as usize][axis];
        if n_axis.abs() < PARALLEL_EPS {
            continue;
        }
        if let Some(coord) = line_tri_hit(mesh, t as usize, axis, a, b) {
            if n_axis < 0.0 {
                fwd.push(coord);
            } else {
                bwd.push(coord);
            }
        }
    }
    fwd.sort_by(|x, y| x.partial_cmp(y).unwrap());
    bwd.sort_by(|x, y| y.partial_cmp(x).unwrap());
    (min_sep(fwd, false), min_sep(bwd, true))
}

/// Merge hits closer than the sensor's 0.02 m step (also removes exact duplicates from
/// triangles sharing an edge on the line). `descending` for backward runs.
fn min_sep(hits: Vec<f64>, descending: bool) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(hits.len());
    for h in hits {
        match out.last() {
            Some(&last) => {
                let gap = if descending { last - h } else { h - last };
                if gap >= MIN_SEP - 1e-9 {
                    out.push(h);
                }
            }
            None => out.push(h),
        }
    }
    out
}

/// Fraction of x/z lines whose FIRST +march hit is back-facing — a sane outward-wound
/// building shows ~0; above 0.5 the winding (or converter handedness) is inverted.
fn backface_first_fraction(mesh: &TriMesh, bins_x: &AxisBins, bins_z: &AxisBins) -> f64 {
    let mut lines = 0usize;
    let mut back_first = 0usize;
    for (axis, bins) in [(0usize, bins_x), (2usize, bins_z)] {
        for j in 0..bins.d1 {
            for k in 0..bins.d2 {
                let a = bins.origin[bins.c1] + (j as f64 + 0.5) * CELL;
                let b = bins.origin[bins.c2] + (k as f64 + 0.5) * CELL;
                let mut first: Option<(f64, f64)> = None; // (coord, n_axis)
                for &t in &bins.bins[j * bins.d2 + k] {
                    if let Some(c) = line_tri_hit(mesh, t as usize, axis, a, b) {
                        if first.is_none_or(|(fc, _)| c < fc) {
                            first = Some((c, mesh.tri_normal[t as usize][axis]));
                        }
                    }
                }
                if let Some((_, n)) = first {
                    lines += 1;
                    if n > PARALLEL_EPS {
                        back_first += 1;
                    }
                }
            }
        }
    }
    if lines == 0 {
        0.0
    } else {
        back_first as f64 / lines as f64
    }
}

/// Generate the dump for a mesh via the shared march skeleton.
pub fn generate(mesh: &TriMesh, ident: DumpIdent) -> VoxelDump {
    let (bbox_min, bbox_max) = xob::aabb(&mesh.verts);
    let origin = [bbox_min[0] - PAD, bbox_min[1] - PAD, bbox_min[2] - PAD];
    let span = [
        bbox_max[0] - bbox_min[0] + 2.0 * PAD,
        bbox_max[1] - bbox_min[1] + PAD + 1.2,
        bbox_max[2] - bbox_min[2] + 2.0 * PAD,
    ];
    let dims = [
        (span[0] / CELL).ceil() as usize,
        (span[1] / CELL).ceil() as usize,
        (span[2] / CELL).ceil() as usize,
    ];
    let bins = [
        AxisBins::build(mesh, 0, origin, dims),
        AxisBins::build(mesh, 1, origin, dims),
        AxisBins::build(mesh, 2, origin, dims),
    ];
    march::generate_dump(ident, bbox_min, bbox_max, |axis, a, b| {
        line_hits(mesh, &bins[axis], axis, a, b)
    })
}

/// Serialize a dump in wire format (meta line, scanlines, end marker), gz when the path
/// ends in `.gz`. Deterministic: axes x→y→z, keys sorted, "+" before "−" per key.
pub fn write_dump(dump: &VoxelDump, path: &std::path::Path) -> Result<usize> {
    let meta = dump.meta.as_ref().context("dump has no meta")?;
    let mut body = String::with_capacity(1 << 20);
    body.push_str(&serde_json::to_string(meta)?);
    body.push('\n');
    let mut lines = 0usize;
    let mut axis_pair = |code_pos: &str,
                         code_neg: &str,
                         pos: &super::types::ScanMap,
                         neg: &super::types::ScanMap,
                         lines: &mut usize| {
        let mut keys: Vec<(usize, usize)> = pos.keys().chain(neg.keys()).copied().collect();
        keys.sort_unstable();
        keys.dedup();
        for (j, k) in keys {
            for (code, map) in [(code_pos, pos), (code_neg, neg)] {
                if let Some(entries) = map.get(&(j, k)) {
                    let vals = serde_json::to_string(entries).expect("f64 vec serializes");
                    body.push_str(&format!("[\"{code}\",{j},{k},{vals}]\n"));
                    *lines += 1;
                }
            }
        }
    };
    axis_pair("x+", "x-", &dump.x_pos, &dump.x_neg, &mut lines);
    axis_pair("z+", "z-", &dump.z_pos, &dump.z_neg, &mut lines);
    axis_pair("y-", "y+", &dump.y_down, &dump.y_up, &mut lines);
    body.push_str(&format!("{{\"end\":{{\"lines\":{lines},\"ms\":0}}}}\n"));

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    if path.extension().is_some_and(|e| e == "gz") {
        let f = fs::File::create(path).with_context(|| path.display().to_string())?;
        let mut gz = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        gz.write_all(body.as_bytes())?;
        gz.finish()?;
    } else {
        fs::write(path, body)?;
    }
    Ok(lines)
}

fn print_stats(m: &xob::XobMesh) {
    if m.descriptors.is_empty() {
        // COLL geometry: no descriptors/materials — report per-record triangle counts.
        let nrec = m.tri_submesh.iter().copied().max().map_or(0, |r| r + 1);
        for r in 0..nrec {
            let n = m.tri_submesh.iter().filter(|&&s| s == r).count();
            println!("  collider record {r}: {n} tris");
        }
        let (min, max) = xob::aabb(&m.verts);
        println!(
            "loaded COLL: {} verts, {} tris, AABB [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}]",
            m.verts.len(),
            m.tris.len(),
            min[0],
            min[1],
            min[2],
            max[0],
            max[1],
            max[2],
        );
        return;
    }
    println!("materials ({}):", m.materials.len());
    for (i, mat) in m.materials.iter().enumerate() {
        println!("  [{i}] {mat}");
    }
    println!("descriptors ({}):", m.descriptors.len());
    for (i, d) in m.descriptors.iter().enumerate() {
        println!(
            "  [{i}] tier {} submesh {} tris {} verts {} stride {} flags 0x{:08X} bbox [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}]",
            d.quality_tier,
            d.submesh_idx,
            d.triangle_count,
            d.unique_verts,
            d.position_stride,
            d.format_flags,
            d.bbox_min[0],
            d.bbox_min[1],
            d.bbox_min[2],
            d.bbox_max[0],
            d.bbox_max[1],
            d.bbox_max[2],
        );
    }
    let (min, max) = xob::aabb(&m.verts);
    println!(
        "loaded tier {}: {} verts, {} tris, AABB [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}]",
        m.tier,
        m.verts.len(),
        m.tris.len(),
        min[0],
        min[1],
        min[2],
        max[0],
        max[1],
        max[2],
    );
}

pub fn run_voxels_from_mesh(args: &[String]) -> Result<u8> {
    let mut mesh_path: Option<PathBuf> = None;
    let mut slug = String::new();
    let mut out_dir: Option<PathBuf> = None;
    let mut resource: Option<String> = None;
    let mut reference: Option<PathBuf> = None;
    let mut axes = AxesRemap::identity();
    let mut flip_winding = false;
    let mut exclude_material: Vec<String> = Vec::new();
    let mut lod: Option<u32> = None;
    let mut stats_only = false;
    let mut geometry = String::from("auto");
    let mut coll_record: Option<u16> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--geometry" if i + 1 < args.len() => {
                geometry = args[i + 1].clone();
                i += 2;
            }
            "--coll-record" if i + 1 < args.len() => {
                coll_record = Some(
                    args[i + 1]
                        .parse()
                        .context("--coll-record wants an index")?,
                );
                i += 2;
            }
            "--mesh" if i + 1 < args.len() => {
                mesh_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--slug" if i + 1 < args.len() => {
                slug = args[i + 1].clone();
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out_dir = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--resource" if i + 1 < args.len() => {
                resource = Some(args[i + 1].clone());
                i += 2;
            }
            "--reference" if i + 1 < args.len() => {
                reference = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--axes" if i + 1 < args.len() => {
                axes = AxesRemap::parse(&args[i + 1])?;
                i += 2;
            }
            "--flip-winding" => {
                flip_winding = true;
                i += 1;
            }
            "--exclude-material" if i + 1 < args.len() => {
                exclude_material.push(args[i + 1].clone());
                i += 2;
            }
            "--lod" if i + 1 < args.len() => {
                lod = Some(args[i + 1].parse().context("--lod wants a tier number")?);
                i += 2;
            }
            "--stats" => {
                stats_only = true;
                i += 1;
            }
            other => bail!("unknown arg '{other}' (see `cargo xtask map --help`)"),
        }
    }
    let mesh_path = mesh_path.context("--mesh <file.xob> is required")?;
    let bytes = fs::read(&mesh_path).with_context(|| mesh_path.display().to_string())?;
    let use_coll = match geometry.as_str() {
        "coll" => true,
        "visual" => false,
        "auto" => xob::has_coll(&bytes),
        other => bail!("--geometry '{other}' is not auto|coll|visual"),
    };
    println!(
        "geometry: {}",
        if use_coll {
            "COLL (fire-collision — the surface LOS traces)"
        } else {
            "LODS (visual mesh)"
        }
    );
    let mut parsed = if use_coll {
        xob::parse_coll(&bytes)?
    } else {
        xob::parse_xob(&bytes, lod)?
    };
    if let Some(rsel) = coll_record {
        let mut tris = Vec::new();
        let mut subs = Vec::new();
        for (i, tri) in parsed.tris.iter().enumerate() {
            if parsed.tri_submesh[i] == rsel {
                tris.push(*tri);
                subs.push(rsel);
            }
        }
        if tris.is_empty() {
            bail!("--coll-record {rsel}: no triangles in that record");
        }
        parsed.tris = tris;
        parsed.tri_submesh = subs;
    }
    print_stats(&parsed);
    if stats_only {
        return Ok(0);
    }
    if slug.is_empty() {
        bail!("--slug <name> is required");
    }

    let mesh = TriMesh::from_xob(&parsed, &axes, flip_winding, &exclude_material);
    let (min, max) = xob::aabb(&mesh.verts);

    if let Some(ref rp) = reference {
        let refd = super::parse::parse_dump(rp)?;
        if let Some(rm) = refd.meta {
            println!(
                "reference bbox  [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}]",
                rm.bbox_min[0],
                rm.bbox_min[1],
                rm.bbox_min[2],
                rm.bbox_max[0],
                rm.bbox_max[1],
                rm.bbox_max[2],
            );
            println!(
                "mesh bbox       [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}]",
                min[0], min[1], min[2], max[0], max[1], max[2],
            );
            let mut worst = 0.0f64;
            for a in 0..3 {
                worst = worst
                    .max((rm.bbox_min[a] - min[a]).abs())
                    .max((rm.bbox_max[a] - max[a]).abs());
            }
            if worst > 0.75 {
                println!(
                    "WARN: frame deviation {worst:.2} m — check --axes (engine bounds include \
                     child entities, so a mesh slightly INSIDE the reference box is normal)"
                );
            } else {
                println!("frame check OK (worst component deviation {worst:.2} m)");
            }
        }
    }

    let winding = {
        let origin = [min[0] - PAD, min[1] - PAD, min[2] - PAD];
        let span = [
            max[0] - min[0] + 2.0 * PAD,
            max[1] - min[1] + PAD + 1.2,
            max[2] - min[2] + 2.0 * PAD,
        ];
        let dims = [
            (span[0] / CELL).ceil() as usize,
            (span[1] / CELL).ceil() as usize,
            (span[2] / CELL).ceil() as usize,
        ];
        let bx = AxisBins::build(&mesh, 0, origin, dims);
        let bz = AxisBins::build(&mesh, 2, origin, dims);
        backface_first_fraction(&mesh, &bx, &bz)
    };
    println!("winding check: back-facing first-hit fraction {winding:.3}");
    if winding > 0.5 {
        println!("WARN: majority back-facing — likely inverted mesh; retry with --flip-winding");
    }

    let ident = DumpIdent {
        slug: slug.clone(),
        resource: resource.unwrap_or_else(|| {
            format!(
                "xob:{}",
                mesh_path.file_name().unwrap_or_default().to_string_lossy()
            )
        }),
    };
    let dump = generate(&mesh, ident);
    let out_dir = out_dir.unwrap_or_else(|| {
        crate::root::find_repo_root()
            .map(|r| r.join("target/mesh-dumps"))
            .unwrap_or_else(|_| PathBuf::from("target/mesh-dumps"))
    });
    let out_path = out_dir.join(format!("{slug}_voxels.jsonl.gz"));
    let lines = write_dump(&dump, &out_path)?;

    // Self-check: the strict dump parser is a free validator of every wire invariant.
    let reparsed = super::parse::parse_dump(&out_path)
        .context("self-check: generated dump failed the strict parser")?;
    let meta = reparsed.meta.as_ref().expect("meta round-trips");
    println!(
        "OK {slug}: {} tris → {} scanlines (dims {}x{}x{}) → {}",
        mesh.tris.len(),
        lines,
        meta.dims[0],
        meta.dims[1],
        meta.dims[2],
        out_path.display()
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Axis-aligned box as 12 outward-wound triangles with matching vertex normals.
    fn cube_mesh(min: [f64; 3], max: [f64; 3]) -> TriMesh {
        let v = |x, y, z| [x, y, z];
        let corners = [
            v(min[0], min[1], min[2]), // 0
            v(max[0], min[1], min[2]), // 1
            v(max[0], max[1], min[2]), // 2
            v(min[0], max[1], min[2]), // 3
            v(min[0], min[1], max[2]), // 4
            v(max[0], min[1], max[2]), // 5
            v(max[0], max[1], max[2]), // 6
            v(min[0], max[1], max[2]), // 7
        ];
        // (indices, outward normal) per face; winding chosen to MATCH the normal so the
        // orientation pass is a no-op for this mesh.
        let faces: [([u32; 4], [f64; 3]); 6] = [
            ([4, 5, 6, 7], [0.0, 0.0, 1.0]),  // +z
            ([1, 0, 3, 2], [0.0, 0.0, -1.0]), // −z
            ([5, 1, 2, 6], [1.0, 0.0, 0.0]),  // +x
            ([0, 4, 7, 3], [-1.0, 0.0, 0.0]), // −x
            ([7, 6, 2, 3], [0.0, 1.0, 0.0]),  // +y
            ([0, 1, 5, 4], [0.0, -1.0, 0.0]), // −y
        ];
        let mut verts = Vec::new();
        let mut tris = Vec::new();
        let mut tri_normal = Vec::new();
        let mut normals = Vec::new();
        for (quad, n) in faces {
            let base = verts.len() as u32;
            for idx in quad {
                verts.push(corners[idx as usize]);
                normals.push(n);
            }
            tris.push([base, base + 1, base + 2]);
            tris.push([base, base + 2, base + 3]);
            tri_normal.push(n);
            tri_normal.push(n);
        }
        TriMesh {
            verts,
            tris,
            tri_normal,
        }
    }

    fn analytic_box_dump(min: [f64; 3], max: [f64; 3]) -> VoxelDump {
        march::generate_dump(
            DumpIdent {
                slug: "t".into(),
                resource: "t://".into(),
            },
            min,
            max,
            |axis, a, b| {
                let (c1, c2) = cross_axes(axis);
                let inside = a >= min[c1] && a < max[c1] && b >= min[c2] && b < max[c2];
                if inside {
                    (vec![min[axis]], vec![max[axis]])
                } else {
                    (vec![], vec![])
                }
            },
        )
    }

    fn mesh_dump(mesh: &TriMesh) -> VoxelDump {
        generate(
            mesh,
            DumpIdent {
                slug: "t".into(),
                resource: "t://".into(),
            },
        )
    }

    #[test]
    fn cube_matches_analytic_box() {
        let (min, max) = ([0.3, 0.2, 0.4], [2.3, 1.7, 2.9]);
        let mesh = cube_mesh(min, max);
        let got = mesh_dump(&mesh);
        let want = analytic_box_dump(min, max);
        let gm = got.meta.as_ref().unwrap();
        let wm = want.meta.as_ref().unwrap();
        assert_eq!(gm.dims, wm.dims);
        assert_eq!(gm.origin, wm.origin);
        for (name, g, w) in [
            ("x+", &got.x_pos, &want.x_pos),
            ("x-", &got.x_neg, &want.x_neg),
            ("y+", &got.y_up, &want.y_up),
            ("y-", &got.y_down, &want.y_down),
            ("z+", &got.z_pos, &want.z_pos),
            ("z-", &got.z_neg, &want.z_neg),
        ] {
            assert_eq!(g.len(), w.len(), "{name}: line count");
            for (key, entries) in w {
                assert_eq!(g.get(key), Some(entries), "{name} line {key:?}");
            }
        }
    }

    #[test]
    fn open_sheet_is_one_sided() {
        // A single −X-facing wall quad at x=1: visible to x+ marches only.
        let verts = vec![
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 2.0],
            [1.0, 2.0, 2.0],
            [1.0, 2.0, 0.0],
        ];
        let n = [-1.0, 0.0, 0.0];
        let mesh = TriMesh {
            verts,
            tris: vec![[0, 1, 2], [0, 2, 3]],
            tri_normal: vec![n, n],
        };
        let dump = mesh_dump(&mesh);
        assert!(!dump.x_pos.is_empty(), "front side must register");
        assert!(
            dump.x_neg.is_empty(),
            "one-sided sheet must not appear in x-"
        );
        // Parallel to y and z marches: no entries at all there.
        assert!(dump.y_up.is_empty() && dump.y_down.is_empty());
        assert!(dump.z_pos.is_empty() && dump.z_neg.is_empty());
    }

    #[test]
    fn wedge_slope_registers_on_vertical_march() {
        // 45° ramp: surface y = x over x∈[0,2], z∈[0,1], normal (−1,1,0)/√2 (up-facing).
        let verts = vec![
            [0.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [2.0, 2.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
        let s = 1.0 / 2.0f64.sqrt();
        let n = [-s, s, 0.0];
        let mesh = TriMesh {
            verts,
            tris: vec![[0, 1, 2], [0, 2, 3]],
            tri_normal: vec![n, n],
        };
        let dump = mesh_dump(&mesh);
        let meta = dump.meta.as_ref().unwrap();
        // A y− (top-down) line at plan (ix, iz) must record entry at y = x(line) with the
        // dump's r2 precision.
        let ix = meta.dims[0] / 2;
        let iz = meta.dims[2] / 2;
        let fx = meta.origin[0] + (ix as f64 + 0.5) * CELL;
        let entry = dump.y_down.get(&(ix, iz)).expect("ramp seen from above");
        let want = march::r2(fx - meta.origin[1]);
        assert!(
            (entry[0] - want).abs() < 1e-9,
            "ramp height: got {} want {want}",
            entry[0]
        );
        // The underside faces −y: nothing enters marching up.
        assert!(dump.y_up.get(&(ix, iz)).is_none());
    }

    #[test]
    fn winding_flip_detected_and_corrected() {
        let (min, max) = ([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let mut inverted = cube_mesh(min, max);
        for n in &mut inverted.tri_normal {
            *n = [-n[0], -n[1], -n[2]];
        }
        let origin = [min[0] - PAD, min[1] - PAD, min[2] - PAD];
        let span = [
            max[0] - min[0] + 2.0 * PAD,
            max[1] - min[1] + PAD + 1.2,
            max[2] - min[2] + 2.0 * PAD,
        ];
        let dims = [
            (span[0] / CELL).ceil() as usize,
            (span[1] / CELL).ceil() as usize,
            (span[2] / CELL).ceil() as usize,
        ];
        let bx = AxisBins::build(&inverted, 0, origin, dims);
        let bz = AxisBins::build(&inverted, 2, origin, dims);
        assert!(backface_first_fraction(&inverted, &bx, &bz) > 0.5);

        // Flipping all normals back restores the analytic-box dump exactly.
        let mut fixed = inverted;
        for n in &mut fixed.tri_normal {
            *n = [-n[0], -n[1], -n[2]];
        }
        let got = mesh_dump(&fixed);
        let want = analytic_box_dump(min, max);
        assert_eq!(got.x_pos.len(), want.x_pos.len());
        assert_eq!(got.y_down.len(), want.y_down.len());
    }

    #[test]
    fn min_sep_merges_close_hits() {
        assert_eq!(min_sep(vec![1.0, 1.0, 1.005, 1.5], false), vec![1.0, 1.5]);
        assert_eq!(min_sep(vec![2.0, 1.995, 1.0], true), vec![2.0, 1.0]);
    }

    #[test]
    fn written_dump_round_trips_through_strict_parser() {
        let mesh = cube_mesh([0.1, 0.0, 0.2], [1.6, 1.2, 1.9]);
        let dump = mesh_dump(&mesh);
        let dir = std::env::temp_dir().join(format!("tbd-meshdump-{}", std::process::id()));
        let path = dir.join("cube_voxels.jsonl.gz");
        let lines = write_dump(&dump, &path).unwrap();
        assert!(lines > 0);
        let re = super::super::parse::parse_dump(&path).unwrap();
        let _ = fs::remove_dir_all(&dir);
        let m0 = dump.meta.as_ref().unwrap();
        let m1 = re.meta.as_ref().unwrap();
        assert_eq!(m0.dims, m1.dims);
        assert_eq!(m0.origin, m1.origin);
        assert_eq!(re.x_pos, dump.x_pos);
        assert_eq!(re.x_neg, dump.x_neg);
        assert_eq!(re.y_up, dump.y_up);
        assert_eq!(re.y_down, dump.y_down);
        assert_eq!(re.z_pos, dump.z_pos);
        assert_eq!(re.z_neg, dump.z_neg);
        assert_eq!(re.truncated, 0);
    }

    #[test]
    fn axes_remap_parses_and_applies() {
        let r = AxesRemap::parse("x,y,-z").unwrap();
        assert_eq!(r.apply([1.0, 2.0, 3.0]), [1.0, 2.0, -3.0]);
        let r = AxesRemap::parse("z,y,x").unwrap();
        assert_eq!(r.apply([1.0, 2.0, 3.0]), [3.0, 2.0, 1.0]);
        assert!(AxesRemap::parse("x,y").is_err());
        assert!(AxesRemap::parse("x,y,w").is_err());
    }
}
