use super::*;

pub(super) fn u16le(p: &[u8]) -> u16 {
    u16::from_le_bytes([p[0], p[1]])
}

pub(super) fn u32le(p: &[u8]) -> u32 {
    u32::from_le_bytes([p[0], p[1], p[2], p[3]])
}

pub(super) fn u32be(p: &[u8]) -> u32 {
    u32::from_be_bytes([p[0], p[1], p[2], p[3]])
}

pub(super) fn f32le(p: &[u8]) -> f32 {
    f32::from_le_bytes([p[0], p[1], p[2], p[3]])
}

pub(super) fn vec3le(p: &[u8]) -> [f32; 3] {
    [f32le(&p[0..4]), f32le(&p[4..8]), f32le(&p[8..12])]
}

/// Byte-scan for an IFF chunk id from offset 12 (after FORM header + form type); payload
/// follows the 4-byte id + big-endian u32 size. Scan-not-iterate mirrors the reference
/// parser: real files carry alignment padding that breaks strict IFF walking.
pub(super) fn find_chunk<'a>(data: &'a [u8], id: &[u8; 4]) -> Option<&'a [u8]> {
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
pub(super) fn parse_materials(head: &[u8]) -> Vec<String> {
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

pub(super) fn parse_descriptors(head: &[u8]) -> Vec<LodDescriptor> {
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
pub(super) fn region<'a>(
    decompressed: &'a [u8],
    descs: &[LodDescriptor],
    i: usize,
) -> Result<&'a [u8]> {
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

pub(super) fn parse_submesh(region: &[u8], d: &LodDescriptor) -> Result<Submesh> {
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

/// Does this xob carry a collision chunk?
pub fn has_coll(data: &[u8]) -> bool {
    data.len() >= 12 && data[0..4] == *b"FORM" && find_chunk(data, b"COLL").is_some()
}

pub(super) fn mat_apply(rot: &[f64; 9], c: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    [
        rot[0] * v[0] + rot[1] * v[1] + rot[2] * v[2] + c[0],
        rot[3] * v[0] + rot[4] * v[1] + rot[5] * v[2] + c[1],
        rot[6] * v[0] + rot[7] * v[1] + rot[8] * v[2] + c[2],
    ]
}
