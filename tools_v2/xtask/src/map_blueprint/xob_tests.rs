//! Tests for [`super`] (the XOB9 reader: LZ4 stream, LODS submeshes, the COLL collider
//! grammar incl. box / convex / plain-trimesh / subranged-trimesh records) — split out per the
//! `#[path]` precedent to stay under the SIZE gate.

use super::*;

/// One COLL box record with an explicit layer-preset name index and first-material
/// index in the header (the T-090.11.2 fields); no subrange table (boxes have none), so
/// the layer preset is the only kind opinion.
pub(crate) fn coll_box_record_with_material(
    center: [f32; 3],
    ext: [f32; 3],
    layer_idx: u16,
    first_mat_idx: u16,
) -> Vec<u8> {
    let mut r = vec![3u8, 0xFF];
    r.extend_from_slice(&layer_idx.to_le_bytes());
    for i in 0..9u32 {
        let v: f32 = if i % 4 == 0 { 1.0 } else { 0.0 };
        r.extend_from_slice(&v.to_le_bytes());
    }
    for c in center {
        r.extend_from_slice(&c.to_le_bytes());
    }
    r.extend_from_slice(&0.0f32.to_le_bytes());
    r.extend_from_slice(&0u16.to_le_bytes());
    r.extend_from_slice(&first_mat_idx.to_le_bytes());
    r.extend_from_slice(&0u32.to_le_bytes());
    for e in ext {
        r.extend_from_slice(&e.to_le_bytes());
    }
    r
}

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

/// One COLL box record: type 3, identity rotation, given center/extents.
fn coll_box_record(center: [f32; 3], ext: [f32; 3]) -> Vec<u8> {
    let mut r = vec![3u8, 0xFF, 1, 0];
    for i in 0..9u32 {
        let v: f32 = if i % 4 == 0 { 1.0 } else { 0.0 };
        r.extend_from_slice(&v.to_le_bytes());
    }
    for c in center {
        r.extend_from_slice(&c.to_le_bytes());
    }
    r.extend_from_slice(&0.0f32.to_le_bytes());
    r.extend_from_slice(&2u16.to_le_bytes());
    r.extend_from_slice(&3u16.to_le_bytes());
    r.extend_from_slice(&0u32.to_le_bytes());
    for e in ext {
        r.extend_from_slice(&e.to_le_bytes());
    }
    r
}

pub(crate) fn with_coll(mut file: Vec<u8>, coll_payload: &[u8]) -> Vec<u8> {
    let mut chunk = b"COLL".to_vec();
    chunk.extend_from_slice(&(coll_payload.len() as u32).to_be_bytes());
    chunk.extend_from_slice(coll_payload);
    file.extend_from_slice(&chunk);
    let total = (file.len() - 8) as u32;
    file[4..8].copy_from_slice(&total.to_be_bytes());
    file
}

#[test]
fn coll_box_record_becomes_twelve_triangles() {
    let file = with_coll(
        tiny_xob(),
        &coll_box_record([1.0, 2.0, 3.0], [0.5, 0.25, 1.5]),
    );
    assert!(has_coll(&file));
    let mesh = parse_coll(&file).unwrap();
    assert_eq!(mesh.verts.len(), 8);
    assert_eq!(mesh.tris.len(), 12);
    let (min, max) = aabb(&mesh.verts);
    assert_eq!(min, [0.5, 1.75, 1.5]);
    assert_eq!(max, [1.5, 2.25, 4.5]);
    assert!(mesh.tri_submesh.iter().all(|&s| s == 0));
    // A box carries no subrange table: the header's first material covers it.
    assert!(mesh.tri_material.iter().all(|&m| m == 3));
    assert_eq!(mesh.records.len(), 1);
    assert_eq!(mesh.records[0].shape, 3);
    assert_eq!(mesh.records[0].layer_idx, 1);
    assert_eq!(
        (mesh.records[0].mesh_idx, mesh.records[0].first_mat_idx),
        (2, 3)
    );
    assert_eq!(
        (mesh.records[0].tri_start, mesh.records[0].tri_count),
        (0, 12)
    );
}

#[test]
fn coll_trimesh_record_parses_and_transforms() {
    // type 6, one triangle, center offset (10, 0, 0).
    let mut r = vec![6u8, 0xFF, 1, 0];
    for i in 0..9u32 {
        let v: f32 = if i % 4 == 0 { 1.0 } else { 0.0 };
        r.extend_from_slice(&v.to_le_bytes());
    }
    for c in [10.0f32, 0.0, 0.0] {
        r.extend_from_slice(&c.to_le_bytes());
    }
    r.extend_from_slice(&0.0f32.to_le_bytes());
    r.extend_from_slice(&2u16.to_le_bytes());
    r.extend_from_slice(&3u16.to_le_bytes());
    r.extend_from_slice(&0u32.to_le_bytes());
    r.extend_from_slice(&4u16.to_le_bytes()); // nv
    r.extend_from_slice(&3u16.to_le_bytes()); // nt
    r.extend_from_slice(&2u32.to_le_bytes()); // nsub
    // Subranges deliberately out of order: material 7 ends at tri 2, material 5 at tri 0.
    r.extend_from_slice(&7u16.to_le_bytes());
    r.extend_from_slice(&2u16.to_le_bytes());
    r.extend_from_slice(&5u16.to_le_bytes());
    r.extend_from_slice(&0u16.to_le_bytes());
    for v in [
        [0.0f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ] {
        for c in v {
            r.extend_from_slice(&c.to_le_bytes());
        }
    }
    for idx in [0u16, 1, 2, 1, 3, 2, 0, 2, 3] {
        r.extend_from_slice(&idx.to_le_bytes());
    }
    let file = with_coll(tiny_xob(), &r);
    let mesh = parse_coll(&file).unwrap();
    assert_eq!(mesh.tris.len(), 3);
    assert_eq!(mesh.verts[1], [11.0, 0.0, 0.0]); // center applied
    assert_eq!(mesh.tri_material, vec![5, 7, 7]);
    assert_eq!(mesh.records[0].layer_idx, 1);
    assert_eq!(mesh.records[0].tri_count, 3);
}

/// Type 4 (convex): a unit cube's eight corners + the undecoded tables, rebuilt as
/// twelve hull triangles; type 5 (plain trimesh): one triangle, material from the header.
#[test]
fn coll_convex_and_plain_trimesh_records_parse() {
    let mut r = vec![4u8, 0xFF, 9, 0];
    for i in 0..9u32 {
        let v: f32 = if i % 4 == 0 { 1.0 } else { 0.0 };
        r.extend_from_slice(&v.to_le_bytes());
    }
    for c in [0.0f32, 5.0, 0.0] {
        r.extend_from_slice(&c.to_le_bytes());
    }
    r.extend_from_slice(&0.0f32.to_le_bytes());
    r.extend_from_slice(&10u16.to_le_bytes());
    r.extend_from_slice(&11u16.to_le_bytes());
    r.extend_from_slice(&0u32.to_le_bytes());
    // 8 verts, 6 faces, 12 edges, 24 indices → tables 4·24 + 4·12 + 4·6 = 168 bytes.
    for c in [8u16, 6, 12, 24] {
        r.extend_from_slice(&c.to_le_bytes());
    }
    for corner in 0..8u32 {
        for c in [
            if corner & 1 != 0 { 0.5f32 } else { -0.5 },
            if corner & 2 != 0 { 0.5 } else { -0.5 },
            if corner & 4 != 0 { 0.5 } else { -0.5 },
        ] {
            r.extend_from_slice(&c.to_le_bytes());
        }
    }
    r.extend_from_slice(&[0xAAu8; 168]);
    // Type 5 right behind it: one triangle, layer 15, material 11.
    r.extend_from_slice(&[5u8, 0xFF, 15, 0]);
    for i in 0..9u32 {
        let v: f32 = if i % 4 == 0 { 1.0 } else { 0.0 };
        r.extend_from_slice(&v.to_le_bytes());
    }
    for c in [0.0f32; 3] {
        r.extend_from_slice(&c.to_le_bytes());
    }
    r.extend_from_slice(&0.0f32.to_le_bytes());
    r.extend_from_slice(&16u16.to_le_bytes());
    r.extend_from_slice(&11u16.to_le_bytes());
    r.extend_from_slice(&0u32.to_le_bytes());
    r.extend_from_slice(&3u16.to_le_bytes());
    r.extend_from_slice(&1u16.to_le_bytes());
    for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]] {
        for c in v {
            r.extend_from_slice(&c.to_le_bytes());
        }
    }
    for idx in [0u16, 1, 2] {
        r.extend_from_slice(&idx.to_le_bytes());
    }
    let file = with_coll(tiny_xob(), &r);
    let mesh = parse_coll(&file).unwrap();
    assert_eq!(mesh.records.len(), 2);
    assert_eq!(mesh.records[0].shape, 4);
    assert_eq!(mesh.records[0].layer_idx, 9);
    assert_eq!(
        (mesh.records[0].tri_start, mesh.records[0].tri_count),
        (0, 12)
    );
    assert_eq!(mesh.verts.len(), 8 + 3);
    let (min, max) = aabb(&mesh.verts[..8]);
    assert_eq!(min, [-0.5, 4.5, -0.5]);
    assert_eq!(max, [0.5, 5.5, 0.5]);
    assert!(mesh.tri_material[..12].iter().all(|&m| m == 11));
    assert_eq!(mesh.records[1].shape, 5);
    assert_eq!(mesh.records[1].layer_idx, 15);
    assert_eq!(
        (mesh.records[1].tri_start, mesh.records[1].tri_count),
        (12, 1)
    );
    assert_eq!(mesh.tri_material[12], 11);
    assert_eq!(mesh.tris[12], [8, 9, 10]);
}

#[test]
fn coll_grammar_drift_is_rejected() {
    let mut payload = coll_box_record([0.0; 3], [1.0; 3]);
    payload.push(0); // trailing garbage → walked != len
    let file = with_coll(tiny_xob(), &payload);
    assert!(parse_coll(&file).is_err());
    assert!(parse_coll(&tiny_xob()).is_err()); // no COLL chunk at all
}
