//! Role: manifest tests.
//! Position: `streaming/loaders/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::loaders::manifest::*;
use serde_json::json;

#[test]
fn parses_objects_block_with_defaults() {
    let raw = json!({ "objects": {
        "prefabsPath": "objects/prefabs.json.gz",
        "chunksPath": "objects/chunks",
        "roadsPath": "objects/roads.json.gz",
        "regionsPath": "objects/forest-regions.json.gz",
        "densityPath": "objects/density",
        "prefabCount": 391,
        "instanceCount": 508291
    }});
    let m = parse_objects_manifest(&raw).unwrap();
    assert_eq!(m.chunk_size_m, DEFAULT_CHUNK_SIZE_M);
    assert_eq!(m.prefab_count, Some(391.0));
    assert_eq!(m.instance_count, Some(508291.0));
    assert_eq!(m.roads_path.as_deref(), Some("objects/roads.json.gz"));
}

#[test]
fn gate_requires_prefabs_and_chunks_paths() {
    assert!(parse_objects_manifest(&json!({ "objects": { "prefabsPath": "p" } })).is_none());
    assert!(parse_objects_manifest(&json!({})).is_none());
}

#[test]
fn narrow_cells_reads_index() {
    let raw = json!({ "cells": [
        { "cx": 10, "cy": 12, "path": "objects/chunks/10_12.json.gz", "instanceCount": 42 },
        { "cx": "x", "cy": 1, "path": "p" }
    ]});
    let cells = narrow_cells(&raw).unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].id, "10_12");
    assert_eq!(cells[0].instance_count, Some(42.0));
    assert!(narrow_cells(&json!({})).is_none());
}

const EVERON_MANIFEST: &str =
    include_str!("../../../../../../../assets_v2/terrains/everon/manifest.json");

fn everon() -> Value {
    serde_json::from_str(EVERON_MANIFEST).expect("committed everon manifest is valid JSON")
}

#[test]
fn everon_manifest_parses_unchanged() {
    let raw = everon();
    let m = parse_objects_manifest(&raw).expect("everon has objects.prefabsPath/chunksPath");
    assert_eq!(m.prefabs_path, "objects/prefabs.json.gz");
    assert_eq!(m.chunks_path, "objects/chunks");
    assert_eq!(m.chunk_size_m, 512.0);
    assert_eq!(m.roads_path.as_deref(), Some("objects/roads.json.gz"));
    assert_eq!(m.density_path.as_deref(), Some("objects/density"));
    assert_eq!(
        m.regions_path.as_deref(),
        Some("objects/forest-regions.json.gz")
    );
    assert_eq!(m.instance_count, Some(1_216_066.0));
    assert_eq!(m.prefab_count, Some(1623.0));
    let objects_bin = m.binary.expect("T-935.13 objects.binary");
    assert!(objects_bin.matches_this_build());
    assert_eq!(objects_bin.chunks, "objects/chunks/{cx}_{cy}.bin");
    assert_eq!(objects_bin.prefabs, "objects/prefabs.rkyv");

    let b = parse_manifest_binary(&raw);
    assert!(b.objects.is_some());
    assert!(b.labels.is_some());
    assert!(b.buildings.is_some());
    assert!(b.dem_raw.is_none(), "dem.raw stays unfilled");
    assert!(b.water.is_none(), "water emitter skipped");
    assert_eq!(satellite_unified_encoding(&raw), Some("tbd-sat-v1"));
    assert!(!b.satellite_unified_v2);
}

#[test]
fn every_binary_block_is_read_when_present() {
    let raw = json!({
        "objects": {
            "prefabsPath": "objects/prefabs.json.gz",
            "chunksPath": "objects/chunks",
            "binary": {
                "schemaVersion": "1.0.0",
                "container": "TBDC", "containerVersion": 1,
                "pod": "ObjectInstancePod", "podBytes": 32,
                "chunks": "objects/chunks/{cx}_{cy}.bin",
                "prefabs": "objects/prefabs.rkyv",
                "roads": "roads/road_network.rkyv",
                "regions": "objects/forest-regions.rkyv",
                "typeInventory": "objects/type-inventory.rkyv"
            }
        },
        "dem": { "raw": { "path": "dem/elevation.dem", "encoding": "tbde-v1" } },
        "labels": { "path": "locations/map_labels.rkyv", "encoding": "rkyv-map-labels-v1" },
        "water": {
            "vectors": "water/water_vectors.rkyv",
            "bathymetry": "water/bathymetry.tbd-bath",
            "encoding": "tbdb-v1"
        },
        "buildings": { "archive": "prefabs/building_blueprints.rkyv", "blas": "prefabs/blas" },
        "tiles": { "satellite": { "unified": { "encoding": "tbd-sat-v2" } } }
    });

    let b = parse_manifest_binary(&raw);
    assert!(!b.is_empty());
    let objects = b.objects.clone().expect("objects.binary");
    assert_eq!(objects.container, TBDC_CONTAINER);
    assert_eq!(objects.pod_bytes, 32);
    assert_eq!(objects.chunks, "objects/chunks/{cx}_{cy}.bin");
    assert_eq!(objects.type_inventory, "objects/type-inventory.rkyv");
    assert_eq!(b.dem_raw.expect("dem.raw").encoding, "tbde-v1");
    assert_eq!(b.labels.expect("labels").path, "locations/map_labels.rkyv");
    assert_eq!(
        b.water.expect("water").bathymetry,
        "water/bathymetry.tbd-bath"
    );
    assert_eq!(b.buildings.expect("buildings").blas, "prefabs/blas");
    assert!(b.satellite_unified_v2);

    let m = parse_objects_manifest(&raw).expect("objects block");
    assert_eq!(m.binary, Some(objects));
}

#[test]
fn pod_shape_mismatch_is_detected() {
    let good = ObjectsBinaryBlock {
        container: TBDC_CONTAINER.to_string(),
        container_version: 1,
        pod: "ObjectInstancePod".to_string(),
        pod_bytes: 32,
        ..ObjectsBinaryBlock::default()
    };
    assert!(good.matches_this_build());

    let mut narrow = good.clone();
    narrow.pod_bytes = 24;
    assert!(
        !narrow.matches_this_build(),
        "24-byte rows are not this POD"
    );

    let mut future = good.clone();
    future.container_version = 2;
    assert!(!future.matches_this_build());

    let mut other = good.clone();
    other.container = "TBDX".to_string();
    assert!(!other.matches_this_build());

    let mut renamed = good;
    renamed.pod = "ObjectInstancePodV2".to_string();
    assert!(!renamed.matches_this_build());
}

#[test]
fn partial_blocks_default_and_malformed_blocks_fall_back() {
    let partial = parse_manifest_binary(&json!({
        "dem": { "raw": { "path": "dem/elevation.dem" } }
    }));
    let raw_block = partial
        .dem_raw
        .expect("dem.raw with a missing field still parses");
    assert_eq!(raw_block.path, "dem/elevation.dem");
    assert_eq!(raw_block.encoding, "", "missing field defaults, not errors");
    assert!(partial.objects.is_none());

    let malformed = parse_manifest_binary(&json!({
        "objects": { "binary": { "podBytes": "thirty-two" } },
        "labels": [1, 2, 3]
    }));
    assert!(malformed.objects.is_none(), "wrong type → JSON fallback");
    assert!(malformed.labels.is_none());
    assert!(malformed.is_empty());
}

#[test]
fn satellite_encoding_absent_is_not_v2() {
    assert_eq!(satellite_unified_encoding(&json!({})), None);
    assert_eq!(
        satellite_unified_encoding(&json!({ "tiles": { "satellite": {} } })),
        None
    );
    assert!(!parse_manifest_binary(&json!({})).satellite_unified_v2);
}
