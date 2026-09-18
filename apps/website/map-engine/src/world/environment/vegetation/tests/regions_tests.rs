//! Role: regions tests.
//! Position: `world/environment/vegetation/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::vegetation::regions::*;
use serde_json::{Value, json};
use std::fs;

fn golden(name: &str) -> Value {
    let path = format!(
        "{}/../../../contracts_v2/fixtures/map/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&fs::read(&path).expect("read golden")).expect("parse golden")
}

#[test]
fn parses_bare_array_golden() {
    let regions = parse_regions_payload(&golden("map-object-regions-everon-sample.json"));
    assert_eq!(regions.len(), 4);
    assert_eq!(regions[0].id, "forest-everon-001");
    assert_eq!(regions[0].kind, "forest");
    assert_eq!(regions[0].tree_count, Some(12400.0));
    assert_eq!(regions[0].polygon.len(), 1);
    assert_eq!(regions[0].polygon[0].len(), 5);
    assert_eq!(regions[3].kind, "waterBody");
}

#[test]
fn accepts_wrapped_and_drops_malformed() {
    let raw = json!({ "regions": [
        { "id": "f1", "kind": "forest", "polygon": [[[0, 0], [1, 0], [1, 1]]] },
        { "id": "bad-kind", "kind": "swamp", "polygon": [[[0, 0], [1, 0], [1, 1]]] },
        { "id": "short-ring", "kind": "field", "polygon": [[[0, 0], [1, 0]]] },
        { "kind": "forest", "polygon": [[[0, 0], [1, 0], [1, 1]]] }
    ]});
    let regions = parse_regions_payload(&raw);
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].id, "f1");
}

#[test]
fn non_payload_is_empty() {
    assert_eq!(parse_regions_payload(&Value::Null).len(), 0);
    assert_eq!(parse_regions_payload(&json!("<html>")).len(), 0);
}

use crate::io::archives::codec::to_bytes;
use crate::streaming::loaders::store::bytes_to_json;
use std::path::PathBuf;

const EVERON_REGIONS: usize = 36;

const EVERON_VERTEX_FLOOR: usize = 2_000;

fn everon_json_regions() -> Vec<LandCoverRegion> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets_v2/terrains/everon/objects/forest-regions.json.gz");
    let raw = fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
    parse_regions_payload(&bytes_to_json(&raw).expect("regions decode"))
}

fn to_archive(regions: &[LandCoverRegion]) -> ForestRegionsArchive {
    ForestRegionsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        regions: regions
            .iter()
            .enumerate()
            .map(|(i, r)| region_to_archive(i, r).expect("region → archive"))
            .collect(),
    }
}

#[test]
fn everon_regions_archive_equals_the_json_regions() {
    let json = everon_json_regions();
    assert_eq!(
        json.len(),
        EVERON_REGIONS,
        "everon region corpus changed; re-pin EVERON_REGIONS deliberately"
    );

    let bytes = to_bytes(&to_archive(&json)).expect("serialise");
    assert!(
        bytes_to_json(&bytes).is_err(),
        "the archive must not be parseable as JSON, or this test has not proven the rkyv \
             route ran"
    );
    assert_ne!(&bytes[..2], &[0x1f, 0x8b], "must not carry gzip magic");

    let back = regions_from_bytes(&bytes).expect("read back");
    assert_eq!(back.len(), json.len());
    #[allow(clippy::cast_possible_truncation)]
    let f32_narrow = |v: Option<f64>| v.map(|n| f64::from(n as f32));
    let mut vertices = 0usize;
    let mut kinds = std::collections::BTreeSet::new();
    for (i, (a, j)) in back.iter().zip(json.iter()).enumerate() {
        assert_eq!(a.id, j.id, "region {i}: id");
        assert_eq!(a.kind, j.kind, "region {i}: kind");
        assert_eq!(
            a.dominant_species_class, j.dominant_species_class,
            "region {i}: dominantSpeciesClass"
        );
        assert_eq!(a.cover_type, j.cover_type, "region {i}: coverType");
        assert_eq!(a.tree_count, j.tree_count, "region {i}: treeCount");
        for (name, got, want) in [
            (
                "densityPerHa",
                a.density_per_ha,
                f32_narrow(j.density_per_ha),
            ),
            ("areaHa", a.area_ha, f32_narrow(j.area_ha)),
        ] {
            assert_eq!(
                got.map(f64::to_bits),
                want.map(f64::to_bits),
                "region {i} ({}): {name}",
                j.id
            );
        }
        assert_eq!(a.polygon.len(), j.polygon.len(), "region {i}: ring count");
        for (k, (ra, rj)) in a.polygon.iter().zip(j.polygon.iter()).enumerate() {
            assert_eq!(ra.len(), rj.len(), "region {i} ring {k}: vertex count");
            for (v, (pa, pj)) in ra.iter().zip(rj.iter()).enumerate() {
                for axis in 0..2 {
                    #[allow(clippy::cast_possible_truncation)]
                    let want = f64::from(pj[axis] as f32);
                    assert_eq!(
                        pa[axis].to_bits(),
                        want.to_bits(),
                        "region {i} ({}) ring {k} vertex {v} axis {axis}",
                        j.id
                    );
                }
            }
            vertices += ra.len();
        }
        kinds.insert(a.kind.clone());
    }
    assert!(
        vertices >= EVERON_VERTEX_FLOOR,
        "only {vertices} ring vertices compared — the corpus is empty or LFS-pointered, so \
             this test proved nothing"
    );
    assert!(!kinds.is_empty(), "no kinds seen: {kinds:?}");
}

#[test]
fn everon_rings_stay_within_f32_of_the_json_metres() {
    let json = everon_json_regions();
    let archive = to_archive(&json);
    let mut worst = 0.0_f64;
    for (a, j) in archive.regions.iter().zip(json.iter()) {
        for (ra, rj) in a.polygon.iter().zip(j.polygon.iter()) {
            for (pa, pj) in ra.iter().zip(rj.iter()) {
                worst = worst
                    .max((f64::from(pa[0]) - pj[0]).abs())
                    .max((f64::from(pa[1]) - pj[1]).abs());
            }
        }
    }
    assert!(worst < 1e-3, "worst vertex drift {worst} m (>= 1 mm)");
}

#[test]
fn wrong_schema_version_is_refused_even_though_the_bytes_validate() {
    let mut archive = to_archive(&everon_json_regions());
    archive.schema_version = ARCHIVE_SCHEMA_VERSION + 1;
    let bytes = to_bytes(&archive).expect("serialise");
    assert!(access_checked::<ForestRegionsArchive>(&bytes).is_ok());
    assert!(matches!(
        regions_from_bytes(&bytes),
        Err(BinaryError::UnsupportedVersion { .. })
    ));
}

#[test]
fn rows_the_json_parser_would_drop_are_refused_on_the_binary_route() {
    let ok = ForestRegion {
        id: "f1".into(),
        kind: "forest".into(),
        polygon: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]],
        tree_count: 4,
        dominant_species_class: String::new(),
        density_per_ha: f32::NAN,
        area_ha: f32::NAN,
        cover_type: String::new(),
    };
    let one = |r: ForestRegion| ForestRegionsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        regions: vec![r],
    };
    assert_eq!(
        regions_from_bytes(&to_bytes(&one(ok.clone())).expect("serialise"))
            .expect("control")
            .len(),
        1
    );
    for (needle, r) in [
        (
            "taxonomy does not know",
            ForestRegion {
                kind: "swamp".into(),
                ..ok.clone()
            },
        ),
        (
            "at least 3",
            ForestRegion {
                polygon: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                ..ok.clone()
            },
        ),
        (
            "no rings",
            ForestRegion {
                polygon: Vec::new(),
                ..ok.clone()
            },
        ),
    ] {
        let bytes = to_bytes(&one(r)).expect("serialise");
        assert!(
            access_checked::<ForestRegionsArchive>(&bytes).is_ok(),
            "the bytes must validate, or this proves nothing about the meaning check"
        );
        let msg = regions_from_bytes(&bytes)
            .expect_err("must refuse")
            .to_string();
        assert!(msg.contains(needle), "expected {needle:?} in: {msg}");
    }
}

#[test]
fn absent_and_present_optionals_round_trip() {
    let regions = vec![
        LandCoverRegion {
            id: "full".into(),
            kind: "field".into(),
            polygon: vec![vec![[0.0, 0.0], [8.0, 0.0], [8.0, 8.0]]],
            tree_count: Some(0.0),
            dominant_species_class: Some("conifer".into()),
            density_per_ha: Some(12.5),
            area_ha: Some(3.25),
            cover_type: Some("grass".into()),
        },
        LandCoverRegion {
            id: "bare".into(),
            kind: "waterBody".into(),
            polygon: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]],
            tree_count: None,
            dominant_species_class: None,
            density_per_ha: None,
            area_ha: None,
            cover_type: None,
        },
    ];
    let bytes = to_bytes(&to_archive(&regions)).expect("serialise");
    let back = regions_from_bytes(&bytes).expect("read back");
    assert_eq!(back, regions, "every optional must survive as itself");
    assert_eq!(back[0].tree_count, Some(0.0), "a real 0 is not \"absent\"");
    assert!(back[1].tree_count.is_none() && back[1].cover_type.is_none());

    for (needle, r) in [
        (
            "empty id",
            LandCoverRegion {
                id: String::new(),
                ..regions[0].clone()
            },
        ),
        (
            "empty coverType",
            LandCoverRegion {
                cover_type: Some(String::new()),
                ..regions[0].clone()
            },
        ),
        (
            "not a u32-representable whole number",
            LandCoverRegion {
                tree_count: Some(1.5),
                ..regions[0].clone()
            },
        ),
        (
            "outside the {forest, field, waterBody}",
            LandCoverRegion {
                kind: "swamp".into(),
                ..regions[0].clone()
            },
        ),
    ] {
        let msg = region_to_archive(0, &r)
            .expect_err("must refuse")
            .to_string();
        assert!(msg.contains(needle), "expected {needle:?} in: {msg}");
    }
}

#[test]
fn malformed_buffers_error_rather_than_yielding_regions() {
    let bytes = to_bytes(&to_archive(&everon_json_regions())).expect("serialise");
    for (what, buf) in [
        ("empty", &[][..]),
        ("truncated tail", &bytes[..bytes.len() - 1]),
        ("shifted head", &bytes[1..]),
        ("json", br#"{"regions":[]}"#),
    ] {
        assert!(regions_from_bytes(buf).is_err(), "{what} was accepted");
    }
}
