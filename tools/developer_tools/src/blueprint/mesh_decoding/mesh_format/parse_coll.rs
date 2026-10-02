use super::*;

/// Parse the COLL chunk into a triangle soup. `tri_submesh` carries the RECORD index so
/// callers can isolate one collider; `vert_normals` are all-zero (collision meshes carry
/// no normal stream — face orientation falls back to index winding).
pub fn parse_coll(data: &[u8]) -> Result<XobMesh> {
    if data.len() < 12 || &data[0..4] != b"FORM" || &data[8..11] != b"XOB" {
        bail!("not a FORM/XOB9 file (magic mismatch)");
    }
    let b = find_chunk(data, b"COLL").context("XOB: no COLL chunk")?;
    let mut mesh = XobMesh {
        verts: Vec::new(),
        vert_normals: Vec::new(),
        tris: Vec::new(),
        tri_submesh: Vec::new(),
        materials: Vec::new(),
        descriptors: Vec::new(),
        tier: 0,
        tri_material: Vec::new(),
        records: Vec::new(),
    };
    let mut p = 0usize;
    let mut rec = 0u16;
    while p + 4 <= b.len() {
        let shape_type = b[p];
        if b[p + 1] != 0xFF {
            bail!(
                "COLL record {rec} at +{p}: framing byte 0x{:02X} != 0xFF",
                b[p + 1]
            );
        }
        let layer_idx = u16le(&b[p + 2..]);
        p += 4;
        if p + 60 > b.len() {
            bail!("COLL record {rec}: truncated fixed part");
        }
        let mut rot = [0.0f64; 9];
        for (i, r) in rot.iter_mut().enumerate() {
            *r = f64::from(f32le(&b[p + i * 4..]));
        }
        let center = {
            let c = vec3le(&b[p + 36..]);
            [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])]
        };
        let mesh_idx = u16le(&b[p + 52..]);
        let first_mat_idx = u16le(&b[p + 54..]);
        p += 60;
        let tri_start = mesh.tris.len();
        match shape_type {
            3 => {
                if p + 12 > b.len() {
                    bail!("COLL record {rec}: truncated box extents");
                }
                let e = vec3le(&b[p..]);
                let e = [f64::from(e[0]), f64::from(e[1]), f64::from(e[2])];
                p += 12;
                // Emit the box as 12 outward-wound triangles in the record frame.
                let base = mesh.verts.len() as u32;
                for corner in 0..8u32 {
                    let local = [
                        if corner & 1 != 0 { e[0] } else { -e[0] },
                        if corner & 2 != 0 { e[1] } else { -e[1] },
                        if corner & 4 != 0 { e[2] } else { -e[2] },
                    ];
                    mesh.verts.push(mat_apply(&rot, center, local));
                    mesh.vert_normals.push([0.0, 0.0, 0.0]);
                }
                // Quads (outward, CCW seen from outside): −x +x −y +y −z +z.
                const QUADS: [[u32; 4]; 6] = [
                    [0, 4, 6, 2],
                    [1, 3, 7, 5],
                    [0, 1, 5, 4],
                    [2, 6, 7, 3],
                    [0, 2, 3, 1],
                    [4, 5, 7, 6],
                ];
                for q in QUADS {
                    mesh.tris.push([base + q[0], base + q[1], base + q[2]]);
                    mesh.tris.push([base + q[0], base + q[2], base + q[3]]);
                    mesh.tri_submesh.push(rec);
                    mesh.tri_submesh.push(rec);
                    mesh.tri_material.push(u32::from(first_mat_idx));
                    mesh.tri_material.push(u32::from(first_mat_idx));
                }
            }
            4 => {
                if p + 8 > b.len() {
                    bail!("COLL record {rec}: truncated convex header");
                }
                let nv = u16le(&b[p..]) as usize;
                let nf = u16le(&b[p + 2..]) as usize;
                let ne = u16le(&b[p + 4..]) as usize;
                let ni = u16le(&b[p + 6..]) as usize;
                p += 8;
                let tables = 4 * ni + 4 * ne + 4 * nf;
                if p + nv * 12 + tables > b.len() {
                    bail!(
                        "COLL record {rec}: convex nv={nv} nf={nf} ne={ne} ni={ni} overruns chunk ({} left)",
                        b.len() - p
                    );
                }
                let local: Vec<[f64; 3]> = (0..nv)
                    .map(|i| {
                        let v = vec3le(&b[p + i * 12..]);
                        [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]
                    })
                    .collect();
                p += nv * 12 + tables;
                let hull = super::super::hull::hull_triangles(&local);
                if hull.is_empty() {
                    bail!("COLL record {rec}: convex collider with {nv} verts spans no volume");
                }
                let base = mesh.verts.len() as u32;
                for v in local {
                    mesh.verts.push(mat_apply(&rot, center, v));
                    mesh.vert_normals.push([0.0, 0.0, 0.0]);
                }
                for t in hull {
                    mesh.tris.push([base + t[0], base + t[1], base + t[2]]);
                    mesh.tri_submesh.push(rec);
                    mesh.tri_material.push(u32::from(first_mat_idx));
                }
            }
            5 => {
                if p + 4 > b.len() {
                    bail!("COLL record {rec}: truncated mesh header");
                }
                let nv = u16le(&b[p..]) as usize;
                let nt = u16le(&b[p + 2..]) as usize;
                p += 4;
                if p + nv * 12 + nt * 6 > b.len() {
                    bail!(
                        "COLL record {rec}: mesh nv={nv} nt={nt} overruns chunk ({} left)",
                        b.len() - p
                    );
                }
                let base = mesh.verts.len() as u32;
                for i in 0..nv {
                    let v = vec3le(&b[p + i * 12..]);
                    mesh.verts.push(mat_apply(
                        &rot,
                        center,
                        [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])],
                    ));
                    mesh.vert_normals.push([0.0, 0.0, 0.0]);
                }
                p += nv * 12;
                for t in 0..nt {
                    let mut tri = [0u32; 3];
                    for (k, slot) in tri.iter_mut().enumerate() {
                        let idx = u16le(&b[p + (t * 3 + k) * 2..]) as u32;
                        if idx as usize >= nv {
                            bail!("COLL record {rec}: index {idx} >= nverts {nv}");
                        }
                        *slot = base + idx;
                    }
                    mesh.tris.push(tri);
                    mesh.tri_submesh.push(rec);
                    mesh.tri_material.push(u32::from(first_mat_idx));
                }
                p += nt * 6;
            }
            6 => {
                if p + 8 > b.len() {
                    bail!("COLL record {rec}: truncated mesh header");
                }
                let nv = u16le(&b[p..]) as usize;
                let nt = u16le(&b[p + 2..]) as usize;
                let nsub = u32le(&b[p + 4..]) as usize;
                if p + 8 + nsub * 4 > b.len() {
                    bail!("COLL record {rec}: truncated subrange table");
                }
                // Subrange table: (material name idx, LAST triangle index of the run),
                // runs back to back from triangle 0 — the farmhouse's tables end at 1128
                // and 2882 for 1129 / 2883 triangles.
                let mut subs: Vec<(u16, usize)> = (0..nsub)
                    .map(|s| {
                        let e = &b[p + 8 + s * 4..];
                        (u16le(e), u16le(&e[2..]) as usize)
                    })
                    .collect();
                subs.sort_by_key(|s| s.1);
                if subs.iter().any(|s| s.1 >= nt.max(1)) {
                    bail!("COLL record {rec}: subrange ends past its {nt} triangles");
                }
                p += 8 + nsub * 4;
                if p + nv * 12 + nt * 6 > b.len() {
                    bail!(
                        "COLL record {rec}: mesh nv={nv} nt={nt} overruns chunk ({} left)",
                        b.len() - p
                    );
                }
                let base = mesh.verts.len() as u32;
                for i in 0..nv {
                    let v = vec3le(&b[p + i * 12..]);
                    mesh.verts.push(mat_apply(
                        &rot,
                        center,
                        [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])],
                    ));
                    mesh.vert_normals.push([0.0, 0.0, 0.0]);
                }
                p += nv * 12;
                let mut run = 0usize;
                for t in 0..nt {
                    let mut tri = [0u32; 3];
                    for (k, slot) in tri.iter_mut().enumerate() {
                        let idx = u16le(&b[p + (t * 3 + k) * 2..]) as u32;
                        if idx as usize >= nv {
                            bail!("COLL record {rec}: index {idx} >= nverts {nv}");
                        }
                        *slot = base + idx;
                    }
                    while run < subs.len() && subs[run].1 < t {
                        run += 1;
                    }
                    let material = subs.get(run).map_or(u32::MAX, |s| u32::from(s.0));
                    mesh.tris.push(tri);
                    mesh.tri_submesh.push(rec);
                    mesh.tri_material.push(material);
                }
                p += nt * 6;
            }
            other => bail!(
                "COLL record {rec} at +{}: unknown shape type {other} — extend the grammar",
                p - 64
            ),
        }
        mesh.records.push(CollRecord {
            shape: shape_type,
            layer_idx,
            mesh_idx,
            first_mat_idx,
            tri_start,
            tri_count: mesh.tris.len() - tri_start,
        });
        rec += 1;
    }
    if p != b.len() {
        bail!("COLL: walked {p} of {} bytes — grammar drift", b.len());
    }
    if mesh.tris.is_empty() {
        bail!("COLL: no colliders decoded");
    }
    Ok(mesh)
}
