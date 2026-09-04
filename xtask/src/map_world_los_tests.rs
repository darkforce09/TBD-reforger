//! Tests for [`super`] — the committed-catalogue pins (T-090.12.3 / .4).

use std::fs;
use std::sync::Arc;

use map_engine_core::building_compound::InstanceKind;
use map_engine_core::bvh::BvhSidecar;
use map_engine_core::geometry::rigid::Rigid;
use map_engine_core::world::occluder::{
    BlockPolicy, PrefabDescriptor, WorldOccluder, map_to_engine,
};
use map_engine_core::world::{TerrainSizeM, WorldChunk};

use super::*;
use crate::map_blueprint::tests::fixture;
use crate::map_parity_report::ParityFile;

fn assets() -> PathBuf {
    crate::root::find_repo_root()
        .unwrap()
        .join("packages/map-assets/everon")
}

/// The T-090.11.4 door-parity oracle replayed through the WORLD occluder: the committed farmhouse
/// descriptor (root shell + every architectural instance, furniture dropped as in the compound
/// pin, doors closed) placed by a synthetic chunk row at a yaw, every local oracle pair mapped
/// through that row's transform. The chunk-row transform + TLAS + trace pipeline must reproduce
/// the compound's (4000, 3983, 0, 17) exactly.
#[test]
fn farmhouse_descriptor_placed_at_a_yaw_replays_the_door_parity_fixture() {
    let prefabs = assets().join("prefabs");
    let d: PrefabDescriptor =
        serde_json::from_str(&fs::read_to_string(prefabs.join("descriptors/132.json")).unwrap())
            .unwrap();
    assert_eq!(d.slug, "FarmHouse_E_1L01_Wood");
    let mut d = d;
    d.instances.retain(|i| i.kind != InstanceKind::Furniture);
    assert_eq!(
        d.instances.len(),
        133,
        "root shell + 132 architectural records"
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
    occ.insert_chunk("1_1", &c);
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
        (4000, 3983, 0, 17)
    );
    assert_eq!(
        replay("FarmHouse_E_1L01_Wood_parity.json"),
        (400, 400, 0, 0)
    );
    // The verdict names the building.
    let r = occ.evaluate_los(rigid.point([-14.0, 1.6, 0.0]), rigid.point([0.0, 1.6, 0.0]));
    assert_ne!(
        r.verdict,
        map_engine_core::world::occluder::WorldVerdict::Clear
    );
    assert!(r.hits[0].id.starts_with("132:1_1:0"), "{}", r.hits[0].id);
}

/// The committed cell 18_0 (the farmhouse village) loads end to end: every placed pid resolves
/// to a descriptor, every BLAS parses, nothing is left as a proxy, and a probe through the
/// farmhouse's own wall is blocked by pid 132.
#[test]
fn cell_18_0_loads_with_no_proxy_rows_and_names_the_farmhouse() {
    let (occ, loaded) = load_cell(&assets(), "18_0").unwrap();
    assert!(loaded.contains(&"18_0".to_string()), "{loaded:?}");
    assert_eq!(occ.proxy_rows("18_0"), Some(0), "every placed pid expanded");
    // Recon rootWorldPos (9363.58, 13.05, 285.60), yaw 38.46: a ray from 14 m west into the origin.
    let r = occ.evaluate_los(
        map_to_engine(9350.0, 285.6, 14.6),
        map_to_engine(9363.58, 285.6, 14.6),
    );
    assert!(r.blocker.as_ref().is_some_and(|b| b.pid == 132), "{r:?}");
}

/// The world-parity pins (T-090.12.4): both cells replayed at their measured agreement. Ignored
/// until the Workbench oracle fixtures land in xtask/tests/fixtures/.
#[test]
#[ignore = "T-090.12.4b: needs xtask/tests/fixtures/world_parity_18_0.json from the Workbench world-parity action"]
fn world_parity_cell_18_0_is_pinned() {
    let (occ, _) = load_cell(&assets(), "18_0").unwrap();
    let file: WorldParityFile =
        serde_json::from_str(&fs::read_to_string(fixture("world_parity_18_0.json")).unwrap())
            .unwrap();
    let r = replay(&occ, &file, BlockPolicy::VISION, &mut Vec::new());
    assert!(r.agreement() >= 0.98, "{r:?}");
}

#[test]
#[ignore = "T-090.12.4b: needs xtask/tests/fixtures/world_parity_forest.json from the Workbench world-parity action"]
fn world_parity_forest_cell_is_pinned() {
    let file: WorldParityFile =
        serde_json::from_str(&fs::read_to_string(fixture("world_parity_forest.json")).unwrap())
            .unwrap();
    let cell = format!("{}_{}", file.cell[0], file.cell[1]);
    let (occ, _) = load_cell(&assets(), &cell).unwrap();
    let r = replay(&occ, &file, BlockPolicy::VISION, &mut Vec::new());
    assert!(r.agreement() >= 0.98, "{r:?}");
}
