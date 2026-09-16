//! Role: chunk bin tests.
//! Position: `streaming/loaders/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::fs;
use std::path::PathBuf;

use crate::environment::buildings::prefab::build_prefab_maps;
use crate::environment::buildings::prefab::narrow_prefab_rows;
use crate::formats::containers::header::HEADER_BYTES;
use crate::formats::pod::instance::POD_BYTES;
use crate::streaming::loaders::chunk::parse_chunk;
use crate::streaming::loaders::chunk_bin::*;
use crate::streaming::loaders::store::bytes_to_json;
use crate::streaming::scheduler::state::IngestOutcome;
use crate::streaming::scheduler::state::WorldResidency;

const EVERON_CHUNKS: usize = 315;

const EVERON_INSTANCE_FLOOR: usize = 1_200_000;

fn everon() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/map-assets/everon")
}

fn encode_by_offset(cx: i16, cy: i16, c: &WorldChunk) -> Vec<u8> {
    let n = c.count as usize;
    let mut out = Vec::with_capacity(HEADER_BYTES + POD_BYTES * n);
    out.extend_from_slice(b"TBDC");
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&(n as u32).to_le_bytes());
    out.extend_from_slice(&cx.to_le_bytes());
    out.extend_from_slice(&cy.to_le_bytes());
    out.extend_from_slice(&[0_u8; 16]);
    for i in 0..n {
        out.extend_from_slice(&c.positions[2 * i].to_le_bytes());
        out.extend_from_slice(&c.positions[2 * i + 1].to_le_bytes());
        out.extend_from_slice(&c.z[i].to_le_bytes());
        out.extend_from_slice(&c.rotations[i].to_le_bytes());
        out.extend_from_slice(&c.pitch[i].to_le_bytes());
        out.extend_from_slice(&c.roll[i].to_le_bytes());
        out.extend_from_slice(&c.scale[i].to_le_bytes());
        out.extend_from_slice(&c.prefab_idx[i].to_le_bytes());
        out.push(c.cls_codes[i]);
        out.push(0);
    }
    assert_eq!(out.len(), HEADER_BYTES + POD_BYTES * n);
    out
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|f| f.to_bits()).collect()
}

fn err_of<E>(r: Result<WorldChunk, E>, msg: &str) -> E {
    match r {
        Ok(c) => panic!("{msg}: got Ok({} rows)", c.count),
        Err(e) => e,
    }
}

fn assert_columns_equal(got: &WorldChunk, want: &WorldChunk, ctx: &str) {
    assert_eq!(got.id, want.id, "{ctx}: id");
    assert_eq!(got.cx, want.cx, "{ctx}: cx");
    assert_eq!(got.cy, want.cy, "{ctx}: cy");
    assert_eq!(got.count, want.count, "{ctx}: count");
    assert_eq!(
        bits(&got.positions),
        bits(&want.positions),
        "{ctx}: positions"
    );
    assert_eq!(got.prefab_idx, want.prefab_idx, "{ctx}: prefab_idx");
    assert_eq!(
        bits(&got.rotations),
        bits(&want.rotations),
        "{ctx}: rotations"
    );
    assert_eq!(bits(&got.z), bits(&want.z), "{ctx}: z");
    assert_eq!(bits(&got.pitch), bits(&want.pitch), "{ctx}: pitch");
    assert_eq!(bits(&got.roll), bits(&want.roll), "{ctx}: roll");
    assert_eq!(bits(&got.scale), bits(&want.scale), "{ctx}: scale");
    assert_eq!(got.cls_codes, want.cls_codes, "{ctx}: cls_codes");
    assert_eq!(
        got.rows_by_class, want.rows_by_class,
        "{ctx}: rows_by_class"
    );
}

fn two_row_chunk() -> WorldChunk {
    WorldChunk {
        id: "-3_4".to_string(),
        cx: -3.0,
        cy: 4.0,
        count: 2,
        positions: vec![4096.5, 8192.25, -1.5, 0.0],
        prefab_idx: vec![1623, 7],
        rotations: vec![271.5, 0.0],
        z: vec![-12.125, 3.0],
        pitch: vec![-3.25, 0.0],
        roll: vec![0.5, 0.0],
        scale: vec![1.75, 1.0],
        cls_codes: vec![2, NO_CLASS],
        rows_by_class: HashMap::from([(2, vec![0])]),
    }
}

#[test]
fn two_row_buffer_round_trips_from_hand_written_bytes() {
    let want = two_row_chunk();
    let bytes = encode_by_offset(-3, 4, &want);
    assert_eq!(bytes.len(), HEADER_BYTES + 2 * POD_BYTES);
    let got = parse_chunk_bin(&bytes).expect("well-formed two-row TBDC");
    assert_columns_equal(&got, &want, "two-row");

    assert_eq!(got.rows_by_class.len(), 1);
}

#[test]
fn empty_chunk_is_zero_rows_not_an_error() {
    let empty = WorldChunk {
        id: "0_0".to_string(),
        ..Default::default()
    };
    let bytes = encode_by_offset(0, 0, &empty);
    assert_eq!(bytes.len(), HEADER_BYTES);
    let got = parse_chunk_bin(&bytes).expect("an empty chunk is a real chunk");
    assert_eq!(got.count, 0);
    assert!(got.positions.is_empty() && got.rows_by_class.is_empty());
}

#[test]
fn truncated_payload_is_err_not_a_short_chunk() {
    let bytes = encode_by_offset(-3, 4, &two_row_chunk());

    let err = err_of(
        parse_chunk_bin(&bytes[..bytes.len() - 1]),
        "must not read short",
    );
    assert!(
        matches!(
            err,
            BinaryError::LengthMismatch {
                expected: 64,
                actual: 63,
                ..
            }
        ),
        "{err}"
    );

    let err = err_of(
        parse_chunk_bin(&bytes[..bytes.len() - POD_BYTES]),
        "row dropped",
    );
    assert!(matches!(err, BinaryError::LengthMismatch { .. }), "{err}");
}

#[test]
fn wrong_magic_is_err() {
    let mut bytes = encode_by_offset(-3, 4, &two_row_chunk());
    bytes[..4].copy_from_slice(b"vers");
    let err = err_of(parse_chunk_bin(&bytes), "LFS pointer is not a chunk");
    assert!(matches!(err, BinaryError::BadMagic { .. }), "{err}");
}

#[test]
fn wrong_version_is_err() {
    let mut bytes = encode_by_offset(-3, 4, &two_row_chunk());
    bytes[4..6].copy_from_slice(&2_u16.to_le_bytes());
    let err = err_of(
        parse_chunk_bin(&bytes),
        "v2 rows may not be read at the v1 stride",
    );
    assert!(
        matches!(err, BinaryError::UnsupportedVersion { actual: 2, .. }),
        "{err}"
    );
}

#[test]
fn buffer_shorter_than_the_header_is_err() {
    let err = err_of(parse_chunk_bin(&[0_u8; 8]), "8 bytes is not a header");
    assert!(matches!(err, BinaryError::Truncated { .. }), "{err}");
    assert!(parse_chunk_bin(&[]).is_err());
}

#[test]
fn overflowing_count_is_err_not_a_wrap() {
    let mut bytes = encode_by_offset(0, 0, &WorldChunk::default());
    bytes[8..12].copy_from_slice(&0x0800_0000_u32.to_le_bytes());
    let err = err_of(
        parse_chunk_bin(&bytes),
        "2^27 rows cannot be in 0 payload bytes",
    );
    assert!(matches!(err, BinaryError::LengthMismatch { .. }), "{err}");
    bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(parse_chunk_bin(&bytes).is_err());
}

#[repr(align(4))]
struct AlignedBuf([u8; 1 + HEADER_BYTES + 2 * POD_BYTES]);

#[test]
fn misaligned_buffer_takes_the_copy_path_and_gives_the_same_columns() {
    let want = two_row_chunk();
    let mut backing = AlignedBuf([0; 1 + HEADER_BYTES + 2 * POD_BYTES]);
    backing.0[1..].copy_from_slice(&encode_by_offset(-3, 4, &want));
    let skewed = &backing.0[1..];

    let payload = &skewed[HEADER_BYTES..];
    assert!(
        matches!(
            instances_from_bytes(payload),
            Err(BinaryError::Misaligned { .. })
        ),
        "expected the zero-copy cast to refuse a 1-mod-4 payload"
    );

    let got = parse_chunk_bin(skewed).expect("the copy path handles a misaligned buffer");
    assert_columns_equal(&got, &want, "misaligned");
}

#[test]
fn id_mismatch_is_rejected_even_though_the_bytes_are_perfect() {
    let bytes = encode_by_offset(-3, 4, &two_row_chunk());

    assert!(parse_chunk_bin(&bytes).is_ok());
    let err = err_of(
        parse_chunk_bin_for("18_0", &bytes),
        "wrong tile must not be accepted",
    );
    assert_eq!(
        err,
        ChunkBinError::IdMismatch {
            requested: "18_0".to_string(),
            header: "-3_4".to_string(),
        }
    );
    assert!(parse_chunk_bin_for("-3_4", &bytes).is_ok());
}

#[test]
fn chunk_bin_path_fills_the_manifest_template() {
    let t = "objects/chunks/{cx}_{cy}.bin";
    assert_eq!(
        chunk_bin_path(t, "18_0").as_deref(),
        Some("objects/chunks/18_0.bin")
    );
    assert_eq!(
        chunk_bin_path(t, "-3_4").as_deref(),
        Some("objects/chunks/-3_4.bin")
    );

    assert_eq!(chunk_bin_path("objects/chunks/all.bin", "18_0"), None);
    assert_eq!(chunk_bin_path("objects/chunks/{cx}.bin", "18_0"), None);

    assert_eq!(chunk_bin_path(t, "a_b"), None);
    assert_eq!(chunk_bin_path(t, ".._.."), None);
    assert_eq!(chunk_bin_path(t, "18"), None);
}

#[test]
fn manifest_decides_the_branch() {
    use crate::streaming::loaders::manifest::parse_manifest_binary;

    let raw = fs::read_to_string(everon().join("manifest.json")).expect("everon manifest");
    let shipped: serde_json::Value = serde_json::from_str(&raw).expect("manifest json");
    let live = parse_manifest_binary(&shipped)
        .objects
        .expect("T-935.13 writes objects.binary");
    assert!(live.matches_this_build());
    assert_eq!(
        chunk_bin_path(&live.chunks, "18_0").as_deref(),
        Some("objects/chunks/18_0.bin")
    );

    let flipped: serde_json::Value = serde_json::json!({ "objects": { "binary": {
        "schemaVersion": "1.0.0", "container": "TBDC", "containerVersion": 1,
        "pod": "ObjectInstancePod", "podBytes": 32,
        "chunks": "objects/chunks/{cx}_{cy}.bin"
    }}});
    let block = parse_manifest_binary(&flipped)
        .objects
        .expect("binary block");
    assert!(
        block.matches_this_build(),
        "this build reads TBDC v1 / 32-byte rows"
    );
    assert_eq!(
        chunk_bin_path(&block.chunks, "18_0").as_deref(),
        Some("objects/chunks/18_0.bin")
    );

    let mut narrow = block.clone();
    narrow.pod_bytes = 24;
    assert!(!narrow.matches_this_build());
}

#[test]
fn everon_chunk_bin_columns_equal_the_gz_decode() {
    let objects = everon().join("objects");
    let prefabs_doc = bytes_to_json(&fs::read(objects.join("prefabs.json.gz")).expect("prefabs"))
        .expect("prefabs decode");
    let (prefab_by_id, _) = build_prefab_maps(narrow_prefab_rows(&prefabs_doc));
    assert!(!prefab_by_id.is_empty(), "empty prefab catalogue");

    let mut files: Vec<PathBuf> = fs::read_dir(objects.join("chunks"))
        .expect("everon objects/chunks")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".json.gz"))
        .collect();
    files.sort();
    assert_eq!(
        files.len(),
        EVERON_CHUNKS,
        "everon chunk corpus changed; re-pin EVERON_CHUNKS deliberately"
    );

    let mut total_rows = 0_usize;
    for path in &files {
        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".json.gz"))
            .expect("chunk file name");
        let raw = bytes_to_json(&fs::read(path).expect("read chunk")).expect(id);
        let want = parse_chunk(id, &raw, &prefab_by_id).expect("gz decode");
        let (cx, cy) = id.split_once('_').expect("chunk id");
        let bytes = encode_by_offset(cx.parse().expect("cx"), cy.parse().expect("cy"), &want);
        let got = parse_chunk_bin_for(id, &bytes).expect("bin decode");
        assert_columns_equal(&got, &want, id);
        total_rows += want.count as usize;
    }
    assert!(
        total_rows >= EVERON_INSTANCE_FLOOR,
        "compared only {total_rows} instances; the corpus is not being read"
    );
}

fn everon_residency() -> WorldResidency {
    let mut r = WorldResidency::new();
    r.load_manifest_json(&fs::read_to_string(everon().join("manifest.json")).expect("manifest"))
        .expect("manifest parse");
    r.load_prefabs_gz(&fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs"))
        .expect("prefabs parse");
    r
}

fn everon_chunk_bytes(id: &str) -> (Vec<u8>, Vec<u8>) {
    let gz = fs::read(
        everon()
            .join("objects/chunks")
            .join(format!("{id}.json.gz")),
    )
    .expect("chunk gz");
    let mut r = everon_residency();
    r.ingest_chunk_gz(id, &gz).expect("gz ingest");
    let chunk = r.chunk(id).expect("chunk resident").clone();
    let (cx, cy) = id.split_once('_').expect("chunk id");
    let bin = encode_by_offset(cx.parse().expect("cx"), cy.parse().expect("cy"), &chunk);
    (gz, bin)
}

#[test]
fn ingest_chunk_bin_matches_ingest_chunk_gz() {
    let id = "18_0";
    let (gz, bin) = everon_chunk_bytes(id);

    let mut via_gz = everon_residency();
    let mut via_bin = everon_residency();
    let a = via_gz.ingest_chunk_gz(id, &gz).expect("gz ingest");
    let b = via_bin.ingest_chunk_bin(id, &bin).expect("bin ingest");

    assert_eq!(a, b, "same disposition");
    assert!(
        matches!(b, IngestOutcome::Applied(n) if n > 0),
        "18_0 is a populated chunk: {b:?}"
    );
    assert_eq!(
        via_gz.resident_instance_count(id),
        via_bin.resident_instance_count(id)
    );
    assert_columns_equal(
        via_bin.chunk(id).expect("bin chunk"),
        via_gz.chunk(id).expect("gz chunk"),
        "residency 18_0",
    );
    assert_eq!(via_bin.chunks_resident(), via_gz.chunks_resident());
}

#[test]
fn ingest_chunk_bin_marks_an_empty_chunk_known_empty_like_the_gz_path() {
    let id = "4_9";
    let gz = br#"{"instances":[]}"#;
    let bin = encode_by_offset(
        4,
        9,
        &WorldChunk {
            id: id.to_string(),
            ..Default::default()
        },
    );
    let mut via_gz = everon_residency();
    let mut via_bin = everon_residency();
    assert_eq!(
        via_gz.ingest_chunk_gz(id, gz).expect("gz"),
        IngestOutcome::ParsedEmpty
    );
    assert_eq!(
        via_bin.ingest_chunk_bin(id, &bin).expect("bin"),
        IngestOutcome::ParsedEmpty
    );

    assert!(via_gz.stats_json().contains("\"known_empty_count\":1"));
    assert!(via_bin.stats_json().contains("\"known_empty_count\":1"));
}

#[test]
fn ingest_chunk_bin_rejects_corrupt_and_mis_served_buffers() {
    let (_, bin) = everon_chunk_bytes("18_0");
    let mut r = everon_residency();

    assert!(r.ingest_chunk_bin("18_0", &bin[..bin.len() - 1]).is_err());
    assert!(r.ingest_chunk_bin("18_0", b"vers").is_err());

    let err = r
        .ingest_chunk_bin("17_0", &bin)
        .expect_err("wrong tile must not be filed under 17_0");
    assert!(matches!(err, ChunkBinError::IdMismatch { .. }), "{err}");
    assert_eq!(r.chunks_resident(), 0, "nothing may be inserted on error");
}
