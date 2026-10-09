//! Tests for [`super`] — the committed-catalogue pins.

use std::fs;
use std::sync::Arc;

use building_interiors::compound::instances::InstanceKind;
use geometry_primitives::rigid_transform::Rigid;
use map_coordinates::chunk_math::TerrainSizeM;
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;
use world_chunks::world_chunk::WorldChunk;
use world_line_of_sight::BlockPolicy;
use world_line_of_sight::WorldOccluder;
use world_line_of_sight::occluder_library::PrefabDescriptor;

use super::*;
use blueprint_compiler::parity_report::ParityFile;
use blueprint_compiler::test_fixtures::fixture;

fn assets() -> PathBuf {
    // The checkout this crate was compiled from, whatever directory the test runs in.
    let root = ::repository_root::find_repository_root().expect("repository root");
    terrain_dir(&root, "everon")
}

/// The door-parity oracle replayed through the WORLD occluder: the committed farmhouse
/// descriptor (root shell + every architectural instance, furniture dropped as in the compound
/// pin, doors closed) placed by a synthetic chunk row at a yaw, every local oracle pair mapped
/// through that row's transform. The chunk-row transform + TLAS + trace pipeline must reproduce
/// the compound's (4000, 3998, 0, 2) exactly, re-blessed from (4000, 3983, 0, 17):
/// the projectile layer policy drops the shell's `Building` physics mesh, and 15 of the 17
/// door-inclusive misses were that mesh disagreeing with the `FireView` fire geometry.
#[test]
fn farmhouse_descriptor_placed_at_a_yaw_replays_the_door_parity_fixture() {
    let prefabs = assets().join("prefabs");
    let d: PrefabDescriptor =
        serde_json::from_str(&fs::read_to_string(prefabs.join("descriptors/132.json")).unwrap())
            .unwrap();
    assert_eq!(d.slug, "FarmHouse_E_1L01_Wood");
    let mut d = d;
    d.instances.retain(|i| i.kind != InstanceKind::Furniture);
    // 133 → 121: the twelve `LightSwitch_02` records sit on the `Prop` preset
    // (no fire geometry) and left the descriptor with the projectile layer policy.
    assert_eq!(
        d.instances.len(),
        121,
        "root shell + 120 architectural records"
    );
    let mut occ = WorldOccluder::new(
        512.0,
        TerrainSizeM {
            width: 12_800.0,
            height: 12_800.0,
        },
    );
    for rel in d.blas_paths() {
        let bytes = fs::read(prefabs.join(rel)).unwrap();
        occ.insert_blas(rel, Arc::new(BvhSidecar::parse(&bytes).unwrap()));
    }
    occ.insert_descriptor(d);
    // One row: map (700, 900), 50 m up, yaw 38.46 (the real farmhouse's heading).
    let mut c = WorldChunk {
        id: "1_1".into(),
        cx: 1.0,
        cy: 1.0,
        count: 1,
        ..Default::default()
    };
    c.positions.extend([700.0_f32, 900.0]);
    c.prefab_idx.push(132);
    c.rotations.push(38.46);
    c.z.push(50.0);
    c.pitch.push(0.0);
    c.roll.push(0.0);
    c.scale.push(1.0);
    c.cls_codes.push(255);
    occ.insert_chunk(&"1_1".into(), &c);
    occ.refresh();
    assert_eq!(occ.expanded_count(), 1);
    assert_eq!(occ.root_kind_of(132), Some(InstanceKind::Shell));
    let rigid = Rigid::from_enfusion(
        [f64::from(700.0_f32), 50.0, f64::from(900.0_f32)],
        [0.0, f64::from(38.46_f32), 0.0],
        1.0,
    );
    let replay = |name: &str| -> (usize, usize, usize, usize) {
        let oracle: ParityFile =
            serde_json::from_str(&fs::read_to_string(fixture(name)).unwrap()).unwrap();
        let (mut agree, mut missed, mut phantom) = (0usize, 0usize, 0usize);
        for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
            let obs = rigid.point([ox, oy, oz]);
            let tgt = rigid.point([tx, ty, tz]);
            let clear = !occ.blocked(obs, tgt, BlockPolicy::VISION);
            if clear == engine_clear {
                agree += 1;
            } else if clear {
                missed += 1;
            } else {
                phantom += 1;
            }
        }
        (oracle.pairs.len(), agree, missed, phantom)
    };
    assert_eq!(
        replay("FarmHouse_E_1L01_Wood_parity_doors.json"),
        (4000, 3998, 0, 2)
    );
    assert_eq!(
        replay("FarmHouse_E_1L01_Wood_parity.json"),
        (400, 400, 0, 0)
    );
    // The verdict names the building.
    let r = occ.evaluate_los(rigid.point([-14.0, 1.6, 0.0]), rigid.point([0.0, 1.6, 0.0]));
    assert_ne!(r.verdict, world_line_of_sight::WorldVerdict::Clear);
    assert!(
        r.hits[0].id.as_str().starts_with("132:1_1:0"),
        "{}",
        r.hits[0].id
    );
}

/// The world-parity pin: the Workbench oracle cell 18_0 (4000 seeded pairs,
/// `EPhysicsLayerPresets.Projectile`, `ENTS` column) replayed through `blocked` under the vision
/// policy on the committed chunks + library, pinned at the measured numbers. Bar: ≥ 98 %.
#[test]
fn world_parity_cell_18_0_is_pinned() {
    let (occ, _) = load_cell_occluder(&assets(), "18_0").unwrap();
    let file: WorldParityFile =
        serde_json::from_str(&fs::read_to_string(fixture("world_parity_18_0.json")).unwrap())
            .unwrap();
    assert_eq!(file.pairs.len(), 4000);
    let r = replay(&occ, &file, BlockPolicy::VISION, &mut Vec::new(), None);
    assert_eq!(
        (r.n, r.agree, r.phantom, r.missed, r.provisional),
        (4000, 3971, 12, 17, 0),
        "{r:?}"
    );
    assert!(r.agreement() >= 0.98, "{r:?}");
}
