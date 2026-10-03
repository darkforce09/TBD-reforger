//! The chunk-ingest binary lane: a `TBDC` chunk ingests into the residency exactly as its gzip
//! JSON twin does.
//!
//! **Role:** unit tests of `ChunkResidency::ingest_chunk_bin` against `ingest_chunk_gz` over the
//! committed Everon export.
//! **Position:** mounted from `crate::chunk_ingest`; reads the fixtures of
//! `world_chunks::test_fixtures` and
//! `prefab_catalog::test_fixtures`.
//! **Signals & state:** none; every case builds its own residencies.
//! **Invariants:** the same disposition and columns from both lanes; nothing is inserted on error.

use std::fs;

use crate::Error;
use crate::state::ChunkResidency;
use crate::state::IngestOutcome;
use prefab_catalog::test_fixtures::everon;
use world_chunks::ChunkId;
use world_chunks::chunk_container::ChunkBinError;
use world_chunks::test_fixtures::assert_columns_equal;
use world_chunks::test_fixtures::encode_by_offset;
use world_chunks::world_chunk::WorldChunk;

fn everon_residency() -> ChunkResidency {
    let mut r = ChunkResidency::new();
    r.load_manifest_json(&fs::read_to_string(everon().join("manifest.json")).expect("manifest"))
        .expect("manifest parse");
    let _glyph_lookup_rebuild = r
        .load_prefabs_gz(&fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs"))
        .expect("prefabs parse");
    r
}

fn everon_chunk_bytes(id: &ChunkId) -> (Vec<u8>, Vec<u8>) {
    let gz = fs::read(
        everon()
            .join("objects/chunks")
            .join(format!("{id}.json.gz")),
    )
    .expect("chunk gz");
    let mut r = everon_residency();
    r.ingest_chunk_gz(id, &gz).expect("gz ingest");
    let chunk = r.chunk(id).expect("chunk resident").clone();
    let (cx, cy) = id.as_str().split_once('_').expect("chunk id");
    let bin = encode_by_offset(cx.parse().expect("cx"), cy.parse().expect("cy"), &chunk);
    (gz, bin)
}

#[test]
fn ingest_chunk_bin_matches_ingest_chunk_gz() {
    let id = &ChunkId::from("18_0");
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
    let id = &ChunkId::from("4_9");
    let gz = br#"{"instances":[]}"#;
    let bin = encode_by_offset(
        4,
        9,
        &WorldChunk {
            id: id.clone(),
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

    assert_eq!(via_gz.known_empty_count(), 1);
    assert_eq!(via_bin.known_empty_count(), 1);
}

#[test]
fn ingest_chunk_bin_rejects_corrupt_and_mis_served_buffers() {
    let tile = ChunkId::from("18_0");
    let (_, bin) = everon_chunk_bytes(&tile);
    let mut r = everon_residency();

    assert!(r.ingest_chunk_bin(&tile, &bin[..bin.len() - 1]).is_err());
    assert!(r.ingest_chunk_bin(&tile, b"vers").is_err());

    let err = r
        .ingest_chunk_bin(&ChunkId::from("17_0"), &bin)
        .expect_err("wrong tile must not be filed under 17_0");
    assert!(
        matches!(err, Error::ChunkContainer(ChunkBinError::IdMismatch { .. })),
        "{err}"
    );
    assert_eq!(r.chunks_resident(), 0, "nothing may be inserted on error");
}
