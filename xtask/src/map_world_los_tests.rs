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
/// the compound's (4000, 3998, 0, 2) exactly — T-090.12.4 re-blessed from (4000, 3983, 0, 17):
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
    // T-090.12.4 — 133 → 121: the twelve `LightSwitch_02` records sit on the `Prop` preset
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
        (4000, 3998, 0, 2)
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

/// The world-parity pins (T-090.12.4): both Workbench oracle cells (4000 seeded pairs each,
/// `EPhysicsLayerPresets.Projectile`, `ENTS` column) replayed through `blocked` under the vision
/// policy on the committed chunks + library, pinned at the measured numbers. Bar: ≥ 98 %.
/// Measured 2026-09-04 (scale re-export + projectile layer policy): village 18_0 3971/4000
/// (99.28 %, 12 phantom / 17 missed), forest 16_2 3977/4000 (99.42 %, 11 / 12). Before the
/// layer policy the same cells scored 3917 and 3897; before the scale re-export 3807 and 3813.
#[test]
fn world_parity_cell_18_0_is_pinned() {
    let (occ, _) = load_cell(&assets(), "18_0").unwrap();
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

#[test]
fn world_parity_forest_cell_is_pinned() {
    let file: WorldParityFile =
        serde_json::from_str(&fs::read_to_string(fixture("world_parity_forest.json")).unwrap())
            .unwrap();
    let cell = format!("{}_{}", file.cell[0], file.cell[1]);
    assert_eq!(cell, "16_2");
    let (occ, _) = load_cell(&assets(), &cell).unwrap();
    let r = replay(&occ, &file, BlockPolicy::VISION, &mut Vec::new(), None);
    assert_eq!(
        (r.n, r.agree, r.phantom, r.missed, r.provisional),
        (4000, 3977, 11, 12, 0),
        "{r:?}"
    );
    assert!(r.agreement() >= 0.98, "{r:?}");
}

/// The engine's foliage semantics, pinned the other way round: making canopy terminal
/// (`--foliage-blocks`) is far worse on both cells, so the vision policy (foliage as
/// concealment) is the one that matches `Projectile`.
#[test]
fn foliage_as_a_blocker_disagrees_with_the_projectile_trace() {
    let file: WorldParityFile =
        serde_json::from_str(&fs::read_to_string(fixture("world_parity_forest.json")).unwrap())
            .unwrap();
    let (occ, _) = load_cell(&assets(), "16_2").unwrap();
    let foliage = BlockPolicy {
        foliage_blocks: true,
        ..BlockPolicy::VISION
    };
    let r = replay(&occ, &file, foliage, &mut Vec::new(), None);
    assert!(r.agreement() < 0.90, "{r:?}");
    assert!(
        r.phantom > 500,
        "canopy blocks the engine never sees: {r:?}"
    );
}

/// The world-inclusive column (`clearWorld` = objects ∧ terrain) against the committed 2 m DEM,
/// reported with its resolution caveat: a floor, not an exact pin (the engine's `WORLD` trace
/// sees terrain detail below 2 m). Skips — loudly — when the DEM is an LFS pointer (CI's
/// selective pull), so the objects-only pins above are the ones that must always run.
#[test]
fn world_parity_world_column_clears_its_floor_when_the_dem_is_present() {
    let dem = match load_dem(&assets()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("world column skipped: DEM not decodable here ({e})");
            return;
        }
    };
    for (name, cell, floor) in [
        ("world_parity_18_0.json", "18_0", 0.96),
        ("world_parity_forest.json", "16_2", 0.94),
    ] {
        let file: WorldParityFile =
            serde_json::from_str(&fs::read_to_string(fixture(name)).unwrap()).unwrap();
        let (occ, _) = load_cell(&assets(), cell).unwrap();
        let r = replay(
            &occ,
            &file,
            BlockPolicy::VISION,
            &mut Vec::new(),
            Some(&dem),
        );
        assert_eq!(r.world_n, 4000);
        assert!(
            r.world_agreement() >= floor,
            "{cell}: world {}/{} ({:.2} %) phantom terrain {} objects {} missed {}",
            r.world_agree,
            r.world_n,
            r.world_agreement() * 100.0,
            r.world_phantom_terrain,
            r.world_phantom_objects,
            r.world_missed
        );
    }
}
