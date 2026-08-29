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
            let mut from = out.len() - offset;
            for _ in 0..mlen {
                let b = out[from];
                out.push(b);
                from += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode `payload` as one literals-only raw LZ4 block (valid: a block may be a single
    /// literal run with no match).
    fn lz4_literal_block(payload: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        let len = payload.len();
        if len < 15 {
            b.push((len as u8) << 4);
        } else {
            b.push(0xF0);
            let mut rest = len - 15;
            while rest >= 255 {
                b.push(255);
                rest -= 255;
            }
            b.push(rest as u8);
        }
        b.extend_from_slice(payload);
        let mut out = (b.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(&b);
        out
    }

    #[test]
    fn lz4_literals_and_match_round_trip() {
        // token 0x32: 3 literals "abc", then match offset 3 len 2+4=6 → "abcabcabc";
        // trailing literals-only sequence "!" ends the block.
        let block = [0x32, b'a', b'b', b'c', 0x03, 0x00, 0x10, b'!'];
        let mut src = (block.len() as u32).to_le_bytes().to_vec();
        src.extend_from_slice(&block);
        let out = lz4_decompress_chained(&src).unwrap();
        assert_eq!(out, b"abcabcabc!");
    }

    #[test]
    fn lz4_match_reaches_across_block_boundary() {
        // Block 1: literals "XYZW". Block 2: 0 literals, match offset 4 len 4 → "XYZW" again,
        // then a literals-only tail "q". The offset points into block 1's output.
        let mut src = lz4_literal_block(b"XYZW");
        let block2 = [0x00, 0x04, 0x00, 0x10, b'q'];
        src.extend_from_slice(&(block2.len() as u32).to_le_bytes());
        src.extend_from_slice(&block2);
        let out = lz4_decompress_chained(&src).unwrap();
        assert_eq!(out, b"XYZWXYZWq");
    }

    #[test]
    fn lz4_overlapping_match_replicates() {
        // "ab" then match offset 1 len 4 → "ab" + "bbbb" (RLE-style overlap).
        let block = [0x20, b'a', b'b', 0x01, 0x00, 0x10, b'.'];
        let mut src = (block.len() as u32).to_le_bytes().to_vec();
        src.extend_from_slice(&block);
        let out = lz4_decompress_chained(&src).unwrap();
        assert_eq!(out, b"abbbbb.");
    }

    /// Build a minimal one-descriptor XOB9: unit-cube-ish quad (4 verts, 2 tris).
    fn tiny_xob() -> Vec<u8> {
        // Region: idx1(12B) + idx2(12B) + positions(4×12B) + normals(4×4B).
        let mut region = Vec::new();
        for idx in [0u16, 1, 2, 0, 2, 3] {
            region.extend_from_slice(&idx.to_le_bytes());
        }
        for idx in [0u16; 6] {
            region.extend_from_slice(&idx.to_le_bytes());
        }
        let verts: [[f32; 3]; 4] = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 3.0, 0.0],
            [0.0, 3.0, 0.0],
        ];
        for v in verts {
            for c in v {
                region.extend_from_slice(&c.to_le_bytes());
            }
        }
        for _ in 0..4 {
            region.extend_from_slice(&[0i8 as u8, 0, 127, 0]); // +Z normals
        }

        // HEAD: one material string + one 116-byte LZO4 descriptor.
        let mut head = Vec::new();
        head.extend_from_slice(b"{00112233AABBCCDD}Assets/Test/quad.emat\0");
        let mut desc = vec![0u8; 116];
        desc[0..4].copy_from_slice(b"LZO4");
        desc[0x04..0x08].copy_from_slice(&0u32.to_le_bytes()); // tier 0
        desc[0x1C..0x20].copy_from_slice(&(region.len() as u32).to_le_bytes());
        desc[0x20..0x24].copy_from_slice(&0x0F00_0002u32.to_le_bytes()); // stride 12, normals
        desc[0x4C..0x4E].copy_from_slice(&2u16.to_le_bytes()); // tris
        desc[0x4E..0x50].copy_from_slice(&4u16.to_le_bytes()); // verts
        desc[0x52..0x54].copy_from_slice(&0u16.to_le_bytes()); // submesh 0
        head.extend_from_slice(&desc);

        let lods = lz4_literal_block(&region);

        let mut file = Vec::new();
        file.extend_from_slice(b"FORM");
        file.extend_from_slice(&0u32.to_be_bytes()); // patched below
        file.extend_from_slice(b"XOB9HEAD");
        // The find_chunk scan starts at 12, so the literal "HEAD" of the form type is
        // followed by its own BE size + payload — same shape as the real container.
        file.extend_from_slice(&(head.len() as u32).to_be_bytes());
        file.extend_from_slice(&head);
        file.extend_from_slice(b"LODS");
        file.extend_from_slice(&(lods.len() as u32).to_be_bytes());
        file.extend_from_slice(&lods);
        let total = (file.len() - 8) as u32;
        file[4..8].copy_from_slice(&total.to_be_bytes());
        file
    }

    #[test]
    fn tiny_xob_parses_to_quad() {
        let file = tiny_xob();
        let mesh = parse_xob(&file, None).unwrap();
        assert_eq!(mesh.verts.len(), 4);
        assert_eq!(mesh.tris.len(), 2);
        assert_eq!(mesh.tris[0], [0, 1, 2]);
        assert_eq!(mesh.tris[1], [0, 2, 3]);
        assert_eq!(mesh.verts[2], [2.0, 3.0, 0.0]);
        assert_eq!(mesh.vert_normals[1], [0.0, 0.0, 1.0]);
        assert_eq!(mesh.materials.len(), 1);
        assert!(mesh.materials[0].contains("quad.emat"));
        let (min, max) = aabb(&mesh.verts);
        assert_eq!(min, [0.0, 0.0, 0.0]);
        assert_eq!(max, [2.0, 3.0, 0.0]);
    }

    #[test]
    fn wrong_magic_is_rejected() {
        assert!(parse_xob(b"NOPE12345678", None).is_err());
        let mut file = tiny_xob();
        file[0] = b'X';
        assert!(parse_xob(&file, None).is_err());
    }
}
