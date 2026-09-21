use super::*;

const CFG: TopoCfg = TopoCfg {
    topo_path: "worlds/Test/Test.topo",
    world_size_m: 12800.0,
};

fn header(section_count: u32, per_section: u32) -> Vec<u8> {
    let mut buf = vec![0u8; HEADER_LEN];
    buf[0x10..0x14].copy_from_slice(&section_count.to_le_bytes());
    buf[0x14..0x18].copy_from_slice(&per_section.to_le_bytes());
    buf
}

fn record(rec_type: u8, verts: &[f32], attrs: &[u32]) -> Vec<u8> {
    let mut out = vec![rec_type];
    out.extend_from_slice(&((verts.len() / 2) as u32).to_le_bytes());
    for v in verts {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&(attrs.len() as u32).to_le_bytes());
    for a in attrs {
        out.extend_from_slice(&a.to_le_bytes());
    }
    out
}

/// A header that declares no sections used to reach `sections.remove(0)` on an empty list and
/// panic. It is a refusal with the file named, not a crash.
#[test]
fn a_topo_declaring_zero_sections_is_an_error_not_a_panic() {
    let err = parse_topo(&header(0, 0), &CFG).expect_err("zero sections");
    let text = format!("{err:#}");
    assert!(text.contains("declares 0 sections"), "{text}");
    assert!(text.contains("worlds/Test/Test.topo"), "{text}");
}

#[test]
fn a_buffer_shorter_than_the_header_is_an_error() {
    let err = parse_topo(&[0u8; 8], &CFG).expect_err("short");
    assert!(format!("{err:#}").contains("shorter than"), "{err:#}");
}

#[test]
fn one_section_with_one_record_decodes_end_to_end() {
    let mut buf = header(1, 1);
    buf.extend(record(TOPO_ROAD_A, &[100.0, 200.0, 300.0, 400.0], &[7]));
    let topo = parse_topo(&buf, &CFG).expect("parses");
    assert_eq!(topo.section_count, 1);
    assert_eq!(topo.per_section, 1);
    assert_eq!(topo.records.len(), 1);
    assert_eq!(topo.records[0].rec_type, TOPO_ROAD_A);
    assert_eq!(topo.records[0].verts, vec![100.0, 200.0, 300.0, 400.0]);
    assert_eq!(topo.records[0].attrs, vec![7]);
    assert_eq!(topo.consumed, buf.len());
    assert_eq!(topo.bytes, buf.len());
}

/// A vertex 2 km outside the world means the record boundary was misjudged; the parser stops
/// there and says where, rather than reading garbage as roads.
#[test]
fn a_record_outside_the_world_stops_the_parse_with_its_offset() {
    let mut buf = header(1, 1);
    buf.extend(record(TOPO_ROAD_A, &[-5000.0, 0.0, 0.0, 0.0], &[]));
    let err = parse_topo(&buf, &CFG).expect_err("out of range");
    assert!(format!("{err:#}").contains("section 0 record 0"), "{err:#}");
}
