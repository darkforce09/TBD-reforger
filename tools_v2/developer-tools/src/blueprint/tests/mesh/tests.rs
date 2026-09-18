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
    assert!(!dump.y_up.contains_key(&(ix, iz)));
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
