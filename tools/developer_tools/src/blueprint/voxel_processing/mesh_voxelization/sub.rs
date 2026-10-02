use super::*;

pub(super) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn cross_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// Möller–Trumbore against an axis-aligned line at cross coords (a, b); returns the axis
/// coordinate of the hit, or None.
pub(super) fn line_tri_hit(mesh: &TriMesh, t: usize, axis: usize, a: f64, b: f64) -> Option<f64> {
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
pub(super) fn line_hits(
    mesh: &TriMesh,
    bins: &AxisBins,
    axis: usize,
    a: f64,
    b: f64,
) -> (Vec<f64>, Vec<f64>) {
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
pub(crate) fn min_sep(hits: Vec<f64>, descending: bool) -> Vec<f64> {
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
pub(super) fn backface_first_fraction(mesh: &TriMesh, bins_x: &AxisBins, bins_z: &AxisBins) -> f64 {
    let mut lines = 0usize;
    let mut back_first = 0usize;
    for (axis, bins) in [(0usize, bins_x), (2usize, bins_z)] {
        for j in 0..bins.d1 {
            for k in 0..bins.d2 {
                let a = bins.origin[bins.c1] + (j as f64 + 0.5) * CELL;
                let b = bins.origin[bins.c2] + (k as f64 + 0.5) * CELL;
                let mut first: Option<(f64, f64)> = None; // (coord, n_axis)
                for &t in &bins.bins[j * bins.d2 + k] {
                    if let Some(c) = line_tri_hit(mesh, t as usize, axis, a, b)
                        && first.is_none_or(|(fc, _)| c < fc)
                    {
                        first = Some((c, mesh.tri_normal[t as usize][axis]));
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
                         pos: &super::super::types::ScanMap,
                         neg: &super::super::types::ScanMap,
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

pub(super) fn print_stats(m: &xob::XobMesh) {
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
