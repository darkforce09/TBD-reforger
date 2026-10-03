//! Unit tests of the `TBDC` container decoder: agreement with the JSON lane, identity checks and the path template.

use crate::chunk_id::ChunkId;
use std::fs;
use std::path::PathBuf;

use crate::chunk_container::*;
use crate::test_fixtures::assert_columns_equal;
use crate::test_fixtures::encode_by_offset;
use crate::world_chunk::parse_chunk;
use prefab_catalog::prefab_rows::build_prefab_maps;
use prefab_catalog::prefab_rows::narrow_prefab_rows;
use prefab_catalog::test_fixtures::everon;
use prefab_catalog::world_payload::bytes_to_json;
use world_file_formats::containers::header::HEADER_BYTES;
use world_file_formats::pod::instance::POD_BYTES;

const EVERON_CHUNKS: usize = 315;

const EVERON_INSTANCE_FLOOR: usize = 1_200_000;

fn err_of<E>(r: Result<WorldChunk, E>, msg: &str) -> E {
    match r {
        Ok(c) => panic!("{msg}: got Ok({} rows)", c.count),
        Err(e) => e,
    }
}

fn two_row_chunk() -> WorldChunk {
    WorldChunk {
        id: ChunkId::from("-3_4"),
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
        id: ChunkId::from("0_0"),
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
        parse_chunk_bin_for(&ChunkId::from("18_0"), &bytes),
        "wrong tile must not be accepted",
    );
    assert_eq!(
        err,
        ChunkBinError::IdMismatch {
            requested: ChunkId::from("18_0"),
            header: ChunkId::from("-3_4"),
        }
    );
    assert!(parse_chunk_bin_for(&ChunkId::from("-3_4"), &bytes).is_ok());
}

#[test]
fn chunk_bin_path_fills_the_manifest_template() {
    let t = "objects/chunks/{cx}_{cy}.bin";
    assert_eq!(
        chunk_bin_path(t, &ChunkId::from("18_0")).as_deref(),
        Some("objects/chunks/18_0.bin")
    );
    assert_eq!(
        chunk_bin_path(t, &ChunkId::from("-3_4")).as_deref(),
        Some("objects/chunks/-3_4.bin")
    );

    assert_eq!(
        chunk_bin_path("objects/chunks/all.bin", &ChunkId::from("18_0")),
        None
    );
    assert_eq!(
        chunk_bin_path("objects/chunks/{cx}.bin", &ChunkId::from("18_0")),
        None
    );

    assert_eq!(chunk_bin_path(t, &ChunkId::from("a_b")), None);
    assert_eq!(chunk_bin_path(t, &ChunkId::from(".._..")), None);
    assert_eq!(chunk_bin_path(t, &ChunkId::from("18")), None);
}

#[test]
fn manifest_decides_the_branch() {
    use crate::terrain_manifest::parse_manifest_binary;

    let raw = fs::read_to_string(everon().join("manifest.json")).expect("everon manifest");
    let shipped: serde_json::Value = serde_json::from_str(&raw).expect("manifest json");
    let live = parse_manifest_binary(&shipped)
        .objects
        .expect("T-935.13 writes objects.binary");
    assert!(live.matches_this_build());
    assert_eq!(
        chunk_bin_path(&live.chunks, &ChunkId::from("18_0")).as_deref(),
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
        chunk_bin_path(&block.chunks, &ChunkId::from("18_0")).as_deref(),
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
    let (prefab_by_id, _) =
        build_prefab_maps(narrow_prefab_rows(&prefabs_doc).expect("Everon ids are whole u32s"));
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
        let chunk_id = ChunkId::from(id);
        let want = parse_chunk(&chunk_id, &raw, &prefab_by_id).expect("gz decode");
        let (cx, cy) = id.split_once('_').expect("chunk id");
        let bytes = encode_by_offset(cx.parse().expect("cx"), cy.parse().expect("cy"), &want);
        let got = parse_chunk_bin_for(&chunk_id, &bytes).expect("bin decode");
        assert_columns_equal(&got, &want, id);
        total_rows += want.count as usize;
    }
    assert!(
        total_rows >= EVERON_INSTANCE_FLOOR,
        "compared only {total_rows} instances; the corpus is not being read"
    );
}
