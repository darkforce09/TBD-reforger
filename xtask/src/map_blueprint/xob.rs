//! Minimal Enfusion XOB9 model reader — just enough to pull triangle geometry out of a
//! Reforger `.xob` for the mesh→voxel-dump generator. Positions, indices, packed vertex
//! normals, and per-submesh material indices; no UVs/tangents/skinning.
//!
//! Format knowledge: the community-reverse-engineered XOB9 layout as implemented by
//! Cyrex0/Enfusion-Unpacker (`src/formats/xob_parser.cpp`, itself following
//! `xob_to_obj.py` from enfusion_toolkit). This is an independent Rust implementation of
//! the documented byte layout; no code was copied.
//!
//! Layout (IFF/FORM container, chunk sizes big-endian):
//! - `FORM <be32 size> XOB9HEAD` file header.
//! - `HEAD` chunk: material resource strings (`{16-hex GUID}path`), then one 116-byte
//!   `LZO4` descriptor per (LOD tier × submesh): quality_tier @+0x04, compressed size
//!   @+0x14, decompressed size @+0x1C, format_flags @+0x20 (upper-byte bit 4 → 16-byte
//!   position stride, else 12), bbox min/max (f32×3) @+0x24/+0x30, triangle_count u16
//!   @+0x4C, unique_verts u16 @+0x4E, submesh_idx u16 @+0x52.
//! - `LODS` chunk: LZ4 block stream — `<le32 header>` per block (size = header & 0x7FFFFFFF,
//!   0 terminates, ≤ 0x20000), raw-LZ4 payload; match offsets reach back across block
//!   boundaries (≤ 64 KiB window), so decoding into one contiguous buffer is exact.
//! - Decompressed stream holds one region per descriptor in REVERSE order (descriptor 0's
//!   region is at the END). Region layout: index array (tri×3 u16) → a second, equal-sized
//!   index array (skipped) → positions (unique_verts × stride, xyz f32 LE) → packed normals
//!   (4 bytes/vertex, i8 xyz ÷ 127).

use anyhow::{Context, Result, bail};

pub struct XobMesh {
    pub verts: Vec<[f64; 3]>,
    /// Per-vertex unit-ish normals from the packed i8 stream (len == verts).
    pub vert_normals: Vec<[f64; 3]>,
    pub tris: Vec<[u32; 3]>,
    /// Per-triangle submesh index (parallel to `tris`) — indexes `materials` when in range.
    pub tri_submesh: Vec<u16>,
    /// Material resource paths in HEAD order.
    pub materials: Vec<String>,
    /// Every descriptor found, for diagnostics (`--stats`).
    pub descriptors: Vec<LodDescriptor>,
    /// The quality tier actually loaded.
    pub tier: u32,
    /// Per-triangle material index in the HEAD name space (COLL trimesh subranges — resolve
    /// through `xob_nodes::XobNodes::name`); `u32::MAX` when the triangle's record carries no
    /// subrange table (box colliders) or the mesh is a visual LOD.
    pub tri_material: Vec<u32>,
    /// COLL collider records in file order (empty for a visual-LOD mesh).
    pub records: Vec<CollRecord>,
}

/// One COLL collider record's header facts (T-090.11.2). `layer_idx` names the layer
/// preset (`Building`, `FireView`, `Glass`, `Foliage`, …) and `mesh_idx` the collider mesh
/// (`UTM_BD_*`), both in the HEAD name space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollRecord {
    pub shape: u8,
    pub layer_idx: u16,
    pub mesh_idx: u16,
    pub first_mat_idx: u16,
    /// Range of this record's triangles in `tris` / `tri_submesh` / `tri_material`.
    pub tri_start: usize,
    pub tri_count: usize,
}

#[derive(Debug, Clone)]
pub struct LodDescriptor {
    pub quality_tier: u32,
    pub decomp_size: u32,
    pub format_flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub triangle_count: u16,
    pub unique_verts: u16,
    pub submesh_idx: u16,
    pub position_stride: usize,
}

fn u16le(p: &[u8]) -> u16 {
    u16::from_le_bytes([p[0], p[1]])
}
fn u32le(p: &[u8]) -> u32 {
    u32::from_le_bytes([p[0], p[1], p[2], p[3]])
}
fn u32be(p: &[u8]) -> u32 {
    u32::from_be_bytes([p[0], p[1], p[2], p[3]])
}
fn f32le(p: &[u8]) -> f32 {
    f32::from_le_bytes([p[0], p[1], p[2], p[3]])
}
fn vec3le(p: &[u8]) -> [f32; 3] {
    [f32le(&p[0..4]), f32le(&p[4..8]), f32le(&p[8..12])]
}

/// Byte-scan for an IFF chunk id from offset 12 (after FORM header + form type); payload
/// follows the 4-byte id + big-endian u32 size. Scan-not-iterate mirrors the reference
/// parser: real files carry alignment padding that breaks strict IFF walking.
fn find_chunk<'a>(data: &'a [u8], id: &[u8; 4]) -> Option<&'a [u8]> {
    let mut pos = 12usize;
    while pos + 8 <= data.len() {
        if &data[pos..pos + 4] == id {
            let size = u32be(&data[pos + 4..pos + 8]) as usize;
            if size > 0 && size < 100_000_000 && pos + 8 + size <= data.len() {
                return Some(&data[pos + 8..pos + 8 + size]);
            }
        }
        pos += 1;
    }
    None
}

/// Material resource strings: `{` + 16 hex + `}` + path bytes until NUL/control.
fn parse_materials(head: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 20 < head.len() {
        if head[i] == b'{'
            && head[i + 1..i + 17].iter().all(u8::is_ascii_hexdigit)
            && head[i + 17] == b'}'
        {
            let start = i + 18;
            let mut end = start;
            while end < head.len() && head[end] >= 32 {
                end += 1;
            }
            if end > start {
                out.push(String::from_utf8_lossy(&head[start..end]).into_owned());
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn parse_descriptors(head: &[u8]) -> Vec<LodDescriptor> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 4 <= head.len() {
        let Some(rel) = head[pos..].windows(4).position(|w| w == b"LZO4") else {
            break;
        };
        let at = pos + rel;
        if at + 0x54 > head.len() {
            break;
        }
        let d = &head[at..];
        let format_flags = u32le(&d[0x20..]);
        out.push(LodDescriptor {
            quality_tier: u32le(&d[0x04..]),
            decomp_size: u32le(&d[0x1C..]),
            format_flags,
            bbox_min: vec3le(&d[0x24..]),
            bbox_max: vec3le(&d[0x30..]),
            triangle_count: u16le(&d[0x4C..]),
            unique_verts: u16le(&d[0x4E..]),
            submesh_idx: u16le(&d[0x52..]),
            position_stride: if (format_flags >> 24) & 0x10 != 0 {
                16
            } else {
                12
            },
        });
        pos = at + 4;
    }
    out
}

/// Decode the LODS LZ4 block stream into one contiguous buffer. Raw LZ4 block format:
/// sequences of `token(hi=literal len, lo=match len−4)`, 255-extension bytes, literals,
/// then `u16 LE offset` + match copy — the last sequence is literals-only. Copying into a
/// single output buffer makes cross-block back-references (the "dictionary chaining" the
/// engine relies on) work with no extra machinery: offsets are ≤ 65535 by format.
pub fn lz4_decompress_chained(src: &[u8]) -> Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(src.len() * 4);
    let mut pos = 0usize;
    while pos + 4 <= src.len() {
        let header = u32le(&src[pos..]);
        pos += 4;
        let block = (header & 0x7FFF_FFFF) as usize;
        if block == 0 || block > 0x20000 || pos + block > src.len() {
            break;
        }
        let end = pos + block;
        let out_start = out.len();
        while pos < end {
            let token = src[pos];
            pos += 1;
            // Literals.
            let mut lit = (token >> 4) as usize;
            if lit == 15 {
                loop {
                    let b = *src.get(pos).context("LZ4: truncated literal length")?;
                    pos += 1;
                    lit += b as usize;
                    if b != 255 {
                        break;
                    }
                }
            }
            if pos + lit > end {
                bail!("LZ4: literal run past block end");
            }
            out.extend_from_slice(&src[pos..pos + lit]);
            pos += lit;
            if pos == end {
                break; // last sequence of the block: literals only
            }
            // Match.
            if pos + 2 > end {
                bail!("LZ4: truncated match offset");
            }
            let offset = u16le(&src[pos..]) as usize;
            pos += 2;
            if offset == 0 || offset > out.len() {
                bail!(
                    "LZ4: match offset {offset} outside window (out={})",
                    out.len()
                );
            }
            let mut mlen = (token & 0x0F) as usize + 4;
            if mlen == 19 {
                loop {
                    let b = *src.get(pos).context("LZ4: truncated match length")?;
                    pos += 1;
                    mlen += b as usize;
                    if b != 255 {
                        break;
                    }
                }
            }
            // Byte-wise copy: overlapping matches (offset < len) replicate by design.
            let start = out.len() - offset;
            for i in 0..mlen {
                let b = out[start + i];
                out.push(b);
            }
        }
        if out.len() - out_start > 0x10000 + 0x20000 {
            bail!(
                "LZ4: block expanded implausibly ({} bytes)",
                out.len() - out_start
            );
        }
        pos = end;
    }
    Ok(out)
}

/// Region for global descriptor index `i`: regions are stored in REVERSE descriptor order,
/// so descriptor 0 owns the final `decomp_size` bytes.
fn region<'a>(decompressed: &'a [u8], descs: &[LodDescriptor], i: usize) -> Result<&'a [u8]> {
    let mut end = decompressed.len();
    for d in &descs[..i] {
        end = end
            .checked_sub(d.decomp_size as usize)
            .context("XOB: LOD regions overrun decompressed stream")?;
    }
    let start = end
        .checked_sub(descs[i].decomp_size as usize)
        .context("XOB: LOD region start underflow")?;
    Ok(&decompressed[start..end])
}

struct Submesh {
    verts: Vec<[f64; 3]>,
    normals: Vec<[f64; 3]>,
    tris: Vec<[u32; 3]>,
}

fn parse_submesh(region: &[u8], d: &LodDescriptor) -> Result<Submesh> {
    let nvert = d.unique_verts as usize;
    let ntri = d.triangle_count as usize;
    let nidx = ntri * 3;
    let idx_bytes = nidx * 2;
    // Two index arrays precede the vertex data; only the first carries the triangles.
    let pos_offset = idx_bytes * 2;
    if pos_offset + nvert * d.position_stride > region.len() {
        bail!(
            "XOB: region too small ({} bytes) for {} tris / {} verts @ stride {}",
            region.len(),
            ntri,
            nvert,
            d.position_stride
        );
    }

    let mut tris = Vec::with_capacity(ntri);
    let mut clamped = 0usize;
    for t in 0..ntri {
        let mut tri = [0u32; 3];
        for (k, slot) in tri.iter_mut().enumerate() {
            let idx = u16le(&region[(t * 3 + k) * 2..]) as u32;
            *slot = if (idx as usize) < nvert {
                idx
            } else {
                clamped += 1;
                0
            };
        }
        tris.push(tri);
    }
    if clamped > 0 {
        eprintln!("  [xob] warn: {clamped} out-of-range indices clamped to 0");
    }

    let mut verts = Vec::with_capacity(nvert);
    for i in 0..nvert {
        let p = &region[pos_offset + i * d.position_stride..];
        let v = vec3le(p);
        verts.push([f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]);
    }

    let normal_offset = pos_offset + nvert * d.position_stride;
    let mut normals = vec![[0.0, 0.0, 1.0]; nvert];
    if normal_offset + nvert * 4 <= region.len() {
        for (i, n) in normals.iter_mut().enumerate() {
            let p = &region[normal_offset + i * 4..];
            *n = [
                f64::from(p[0] as i8) / 127.0,
                f64::from(p[1] as i8) / 127.0,
                f64::from(p[2] as i8) / 127.0,
            ];
        }
    }
    Ok(Submesh {
        verts,
        normals,
        tris,
    })
}

/// Parse a `.xob`, loading every submesh of one quality tier (default: the numerically
/// lowest tier present — the full-detail LOD0).
pub fn parse_xob(data: &[u8], tier: Option<u32>) -> Result<XobMesh> {
    if data.len() < 12 || &data[0..4] != b"FORM" || &data[8..11] != b"XOB" {
        bail!("not a FORM/XOB9 file (magic mismatch)");
    }
    let head = find_chunk(data, b"HEAD").context("XOB: no HEAD chunk")?;
    let materials = parse_materials(head);
    let descriptors = parse_descriptors(head);
    if descriptors.is_empty() {
        bail!("XOB: no LZO4 descriptors in HEAD");
    }
    let lods = find_chunk(data, b"LODS").context("XOB: no LODS chunk")?;
    let decompressed = lz4_decompress_chained(lods)?;
    let total: usize = descriptors.iter().map(|d| d.decomp_size as usize).sum();
    if total > decompressed.len() {
        bail!(
            "XOB: descriptors claim {} bytes but LODS decompressed to {}",
            total,
            decompressed.len()
        );
    }

    // Default tier = the one carrying the most triangles: on real assets the full-detail
    // LOD is the HIGHEST tier number (FarmHouse: tier 4 = 22.9k tris, tier 1 = 87-tri hull).
    let tier = tier.unwrap_or_else(|| {
        let mut sums: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
        for d in &descriptors {
            if d.triangle_count > 0 && d.unique_verts > 0 {
                *sums.entry(d.quality_tier).or_default() += u64::from(d.triangle_count);
            }
        }
        sums.into_iter()
            .max_by_key(|&(tier, sum)| (sum, tier))
            .map_or(0, |(t, _)| t)
    });

    let mut mesh = XobMesh {
        verts: Vec::new(),
        vert_normals: Vec::new(),
        tris: Vec::new(),
        tri_submesh: Vec::new(),
        materials,
        descriptors: descriptors.clone(),
        tier,
        tri_material: Vec::new(),
        records: Vec::new(),
    };
    for (i, d) in descriptors.iter().enumerate() {
        if d.quality_tier != tier || d.triangle_count == 0 || d.unique_verts == 0 {
            continue;
        }
        let reg = region(&decompressed, &descriptors, i)?;
        let sub = parse_submesh(reg, d)
            .with_context(|| format!("descriptor {i} (tier {} submesh {})", tier, d.submesh_idx))?;
        let base = mesh.verts.len() as u32;
        mesh.verts.extend(sub.verts);
        mesh.vert_normals.extend(sub.normals);
        for t in sub.tris {
            mesh.tris.push([t[0] + base, t[1] + base, t[2] + base]);
            mesh.tri_submesh.push(d.submesh_idx);
            mesh.tri_material.push(u32::MAX);
        }
    }
    if mesh.tris.is_empty() {
        bail!("XOB: tier {tier} yielded no triangles");
    }
    Ok(mesh)
}

pub fn aabb(verts: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    for v in verts {
        for a in 0..3 {
            min[a] = min[a].min(v[a]);
            max[a] = max[a].max(v[a]);
        }
    }
    (min, max)
}

/* ───────────────────────────── COLL: the collision chunk ─────────────────────────────
 *
 * Reverse-engineered in this repo (2026-08-29) from CardboardBox_01.xob (one 76-byte box
 * record, half-extents match the visual AABB) and FarmHouse_E_1L01.xob (two trimesh
 * records, 62,332 bytes consumed byte-exact; vertex AABB reproduces the engine dump's
 * bounds to the centimeter, chimney included). No public parser existed for this chunk.
 *
 * COLL payload = sequence of collider records, each:
 *   u8 shape_type · u8 0xFF · u16 layer_idx (layer-preset NAME in the HEAD name space:
 *   `Building`, `FireView`, `Glass`, `Foliage`, … — T-090.11.2) ·
 *   rotation 3×3 f32 (row-major) · center 3×f32 · f32 0 · u16 pair (mesh name idx,
 *   first material idx) · u32 0 ·
 *   shape payload:
 *     type 3 (box):     half-extents 3×f32
 *     type 4 (convex):  u16 nverts · u16 nfaces · u16 nedges · u16 nidx · verts nverts×3×f32
 *                       · face/edge tables of 2·nidx·2 + 4·nedges + 4·nfaces bytes (undecoded;
 *                       the hull is rebuilt from the vertices, see `hull.rs`) — a conifer's
 *                       `UCX_C` trunk (10 verts, 208 table bytes) and `UCX_Fol` canopy
 *                       (19 verts, 748 bytes) pin the stride
 *     type 5 (trimesh): u16 nverts · u16 ntris · verts nverts×3×f32 · indices ntris×3×u16
 *                       (no subrange table — the header's first material covers every tri;
 *                       a conifer's `UTM_F` fire geometry, 315 verts / 619 tris)
 *     type 6 (trimesh): u16 nverts · u16 ntris · u32 nsub · nsub×(u16 material, u16 last_tri)
 *                       · verts nverts×3×f32 · indices ntris×3×u16
 * The subrange table is the per-triangle game material: each entry names a
 * `Common/Materials/Game/<stem>.gamemat` (same name space as the node records, see
 * `xob_nodes.rs`) for the run of triangles ending at `last_tri` (inclusive; runs are back
 * to back from triangle 0) — the farmhouse's record 0 carries nine (tiles_ceramic … brick)
 * ending at 8, 18, 78, …, 1128 for its 1129 triangles. `VOLM` stays unparsed: the layer
 * preset lives in the record header.
 */

/// Does this xob carry a collision chunk?
pub fn has_coll(data: &[u8]) -> bool {
    data.len() >= 12 && data[0..4] == *b"FORM" && find_chunk(data, b"COLL").is_some()
}

fn mat_apply(rot: &[f64; 9], c: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    [
        rot[0] * v[0] + rot[1] * v[1] + rot[2] * v[2] + c[0],
        rot[3] * v[0] + rot[4] * v[1] + rot[5] * v[2] + c[1],
        rot[6] * v[0] + rot[7] * v[1] + rot[8] * v[2] + c[2],
    ]
}

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
                let hull = super::hull::hull_triangles(&local);
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

#[cfg(test)]
#[path = "xob_tests.rs"]
pub(crate) mod tests;
