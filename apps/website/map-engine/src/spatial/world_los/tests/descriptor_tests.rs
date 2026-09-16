//! Role: descriptor tests.
//! Position: `spatial/world_los/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::compound::assembly::CoverTier;
use crate::architecture::compound::assembly::PlacementSource;
use crate::architecture::compound::instances::InstanceKind;
use crate::architecture::compound::instances::LocalTransform;
use crate::spatial::world_los::descriptor::*;

fn record(id: &str, kind: InstanceKind, blas: &str) -> InstanceRecord {
    InstanceRecord {
        id: id.into(),
        kind,
        prefab: "Prefabs/X.et".into(),
        blas: blas.into(),
        xob: None,
        local: LocalTransform::identity(),
        door: None,
        cover: CoverTier::None,
        source: PlacementSource::PrefabCoords,
        parent: None,
    }
}

#[test]
fn descriptor_round_trips_and_lists_distinct_blas_in_first_use_order() {
    let d = PrefabDescriptor {
        schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
        prefab_id: 7,
        slug: "X".into(),
        resource_name: "Prefabs/X.et".into(),
        kind: "building".into(),
        blocks: true,
        reason: None,
        canopy: false,
        local_bounds: Some(Bounds3 {
            min: [-1.0, 0.0, -2.0],
            max: [1.0, 3.0, 2.0],
        }),
        shell_bvh: "blas/x.bvh".into(),
        instances: vec![
            record("X", InstanceKind::Shell, "blas/x.bvh"),
            record("X/door", InstanceKind::DoorLeaf, "blas/door.bvh"),
            record("X/door2", InstanceKind::DoorLeaf, "blas/door.bvh"),
        ],
        notes: vec![],
    };
    let json = serde_json::to_string(&d).unwrap();
    assert!(json.contains("\"schemaVersion\":\"1.0.0\""));
    assert!(json.contains("\"prefabId\":7"));
    assert!(!json.contains("\"reason\""), "None reason is omitted");
    assert!(!json.contains("\"notes\""), "empty notes are omitted");
    let back: PrefabDescriptor = serde_json::from_str(&json).unwrap();
    assert_eq!(back, d);
    assert_eq!(back.blas_paths(), vec!["blas/x.bvh", "blas/door.bvh"]);
}

#[test]
fn blocks_false_descriptor_carries_its_reason_and_no_blas() {
    let d = PrefabDescriptor {
        schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
        prefab_id: 9,
        slug: "Decal".into(),
        resource_name: "Prefabs/Decal.et".into(),
        kind: "prop".into(),
        blocks: false,
        reason: Some("no-coll".into()),
        canopy: false,
        local_bounds: None,
        shell_bvh: String::new(),
        instances: vec![],
        notes: vec!["Decal: Assets/decal.xob has no collision chunk".into()],
    };
    let back: PrefabDescriptor = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
    assert_eq!(back, d);
    assert!(back.blas_paths().is_empty());
}

#[test]
fn manifest_lookups_are_binary_searches_over_sorted_entries() {
    let m = BlasManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.into(),
        terrain_id: "everon".into(),
        blas: vec![
            BlasEntry {
                path: "blas/a.bvh".into(),
                bytes: 10,
                tris: 1,
                kinds: [1, 0, 0],
            },
            BlasEntry {
                path: "blas/b.bvh".into(),
                bytes: 20,
                tris: 2,
                kinds: [0, 0, 2],
            },
        ],
        descriptors: vec![
            DescEntry {
                pid: 3,
                path: "descriptors/3.json".into(),
                kind: "tree".into(),
                blocks: true,
                canopy: true,
                blas: vec!["blas/b.bvh".into()],
                instance_count: 1,
                instances_in_world: 500,
            },
            DescEntry {
                pid: 8,
                path: "descriptors/8.json".into(),
                kind: "prop".into(),
                blocks: false,
                canopy: false,
                blas: vec![],
                instance_count: 0,
                instances_in_world: 2,
            },
        ],
        hot: vec![3],
        totals: Totals::default(),
    };
    assert_eq!(m.descriptor(3).map(|d| d.instances_in_world), Some(500));
    assert_eq!(m.descriptor(4), None);
    assert_eq!(m.blas("blas/b.bvh").map(|b| b.tris), Some(2));
    assert_eq!(m.blas("blas/c.bvh"), None);
    let back: BlasManifest = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_eq!(back, m);
}

use crate::formats::archives::blueprints::BuildingBlueprintArchive;
use crate::formats::archives::codec::to_bytes;
use crate::formats::archives::version::ARCHIVE_SCHEMA_VERSION;

fn library() -> Vec<BlasEntry> {
    vec![
        BlasEntry {
            path: "blas/x.bvh".into(),
            bytes: 10,
            tris: 1,
            kinds: [1, 0, 0],
        },
        BlasEntry {
            path: "blas/door.bvh".into(),
            bytes: 20,
            tris: 2,
            kinds: [2, 0, 0],
        },
    ]
}

fn index_of(lib: &[BlasEntry]) -> impl Fn(&str) -> Option<u32> + '_ {
    move |p: &str| {
        lib.iter()
            .position(|e| e.path == p)
            .and_then(|i| u32::try_from(i).ok())
    }
}

fn archived(d: &PrefabDescriptor, lib: &[BlasEntry]) -> BuildingArchiveBytes {
    let a = BuildingBlueprintArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        descriptors: vec![d.to_archived(&index_of(lib)).expect("project")],
        blas_index: lib.iter().map(BlasEntry::to_archived).collect(),
        blueprints: vec![],
    };
    BuildingArchiveBytes::new(&to_bytes(&a).expect("serialise"))
}

#[test]
fn a_future_archive_schema_is_refused_rather_than_read() {
    let future = BuildingBlueprintArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION + 1,
        descriptors: vec![],
        blas_index: vec![],
        blueprints: vec![],
    };
    let bytes = BuildingArchiveBytes::new(&to_bytes(&future).expect("serialise"));
    let err = bytes
        .archive()
        .expect_err("a schema this build does not implement must not be read");
    println!("── refusal ── {err}");
    assert!(
        matches!(err, BinaryError::UnsupportedVersion { .. }),
        "and it must say so, not blame the layout: {err:?}"
    );

    let current = BuildingBlueprintArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        descriptors: vec![],
        blas_index: vec![],
        blueprints: vec![],
    };
    let ok = BuildingArchiveBytes::new(&to_bytes(&current).expect("serialise"));
    assert!(ok.archive().is_ok());
}

#[test]
fn archived_descriptor_round_trips_to_its_census_and_keeps_the_blas_order() {
    let d = PrefabDescriptor {
        schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
        prefab_id: 7,
        slug: "X".into(),
        resource_name: "Prefabs/X.et".into(),
        kind: "building".into(),
        blocks: true,
        reason: None,
        canopy: false,
        local_bounds: Some(Bounds3 {
            min: [-1.5, 0.0, -2.25],
            max: [1.5, 3.0, 2.25],
        }),
        shell_bvh: "blas/x.bvh".into(),
        instances: vec![
            record("X", InstanceKind::Shell, "blas/x.bvh"),
            record("X/door", InstanceKind::DoorLeaf, "blas/door.bvh"),
            record("X/door2", InstanceKind::DoorLeaf, "blas/door.bvh"),
        ],
        notes: vec![],
    };
    let lib = library();
    let held = archived(&d, &lib);
    let a = held.archive().expect("access_checked validates");
    let row = &a.descriptors[0];

    assert_eq!(PrefabDescriptor::from_archived(row), d.archive_census());

    assert_eq!(
        PrefabDescriptor::archived_blas_paths(row, &a.blas_index),
        Some(
            d.blas_paths()
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        BlasEntry::from_archived(&a.blas_index[1]),
        lib[1],
        "the library index is lossless"
    );
}

#[test]
fn a_non_blocking_descriptor_archives_with_no_bounds_and_no_blas() {
    let d = PrefabDescriptor {
        schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
        prefab_id: 9,
        slug: "Decal".into(),
        resource_name: "Prefabs/Decal.et".into(),
        kind: "prop".into(),
        blocks: false,
        reason: Some("no-coll".into()),
        canopy: false,
        local_bounds: None,
        shell_bvh: String::new(),
        instances: vec![],
        notes: vec!["Decal: no collision chunk".into()],
    };
    let lib = library();
    let held = archived(&d, &lib);
    let a = held.archive().expect("access");
    let back = PrefabDescriptor::from_archived(&a.descriptors[0]);
    assert_eq!(back, d.archive_census());
    assert!(!back.blocks);
    assert_eq!(back.local_bounds, None, "blocks: false carries no bounds");
    assert_eq!(
        PrefabDescriptor::archived_blas_paths(&a.descriptors[0], &a.blas_index),
        Some(vec![])
    );
}

#[test]
fn projection_refuses_bounds_that_disagree_with_blocks_and_an_unknown_blas() {
    let mut d = PrefabDescriptor {
        schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
        prefab_id: 4,
        slug: "Y".into(),
        resource_name: "Prefabs/Y.et".into(),
        kind: "prop".into(),
        blocks: true,
        reason: None,
        canopy: false,
        local_bounds: None,
        shell_bvh: "blas/x.bvh".into(),
        instances: vec![record("Y", InstanceKind::Prop, "blas/x.bvh")],
        notes: vec![],
    };
    let lib = library();
    assert_eq!(
        d.to_archived(&index_of(&lib)),
        Err(ArchiveProjectionError::BoundsDisagreeWithBlocks {
            prefab_id: 4,
            blocks: true
        })
    );
    d.local_bounds = Some(Bounds3 {
        min: [0.0; 3],
        max: [1.0; 3],
    });
    d.instances = vec![record("Y", InstanceKind::Prop, "blas/missing.bvh")];
    assert_eq!(
        d.to_archived(&index_of(&lib)),
        Err(ArchiveProjectionError::UnknownBlas {
            prefab_id: 4,
            path: "blas/missing.bvh".into()
        })
    );
    assert!(
        !ArchiveProjectionError::UnknownBlas {
            prefab_id: 4,
            path: "blas/missing.bvh".into()
        }
        .to_string()
        .is_empty()
    );
}

#[test]
fn aligned_holder_preserves_bytes_and_reads_a_misaligned_source() {
    let src: Vec<u8> = (0u8..=250).collect();
    let held = BuildingArchiveBytes::new(&src);
    assert_eq!(held.as_slice(), &src[..], "251 bytes, not a multiple of 8");
    assert_eq!(
        held.as_slice().as_ptr() as usize % 8,
        0,
        "the archive buffer must be 8-aligned"
    );
    let d = PrefabDescriptor {
        schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
        prefab_id: 1,
        slug: "Z".into(),
        resource_name: String::new(),
        kind: "prop".into(),
        blocks: false,
        reason: None,
        canopy: false,
        local_bounds: None,
        shell_bvh: String::new(),
        instances: vec![],
        notes: vec![],
    };
    let lib = library();
    let bytes = archived(&d, &lib).as_slice().to_vec();

    let mut shifted = vec![0u8];
    shifted.extend_from_slice(&bytes);
    let held = BuildingArchiveBytes::new(&shifted[1..]);
    assert_eq!(
        held.archive()
            .expect("re-aligned copy validates")
            .descriptors[0]
            .slug
            .as_str(),
        "Z"
    );
    assert!(
        BuildingArchiveBytes::new(&bytes[..bytes.len() - 4])
            .archive()
            .is_err(),
        "a truncated archive is an error, never a wild read"
    );
}

#[test]
fn bounds_union_is_componentwise() {
    let a = Bounds3 {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    let b = Bounds3 {
        min: [-1.0, 0.5, 0.0],
        max: [0.5, 2.0, 3.0],
    };
    assert_eq!(
        a.union(b),
        Bounds3 {
            min: [-1.0, 0.0, 0.0],
            max: [1.0, 2.0, 3.0]
        }
    );
}
