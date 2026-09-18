use super::*;
use crate::blueprint::tests::fixture;
use crate::repository_layout::terrain_dir;

/// The T-090.11.4 door-parity pin: the committed shell + every architectural instance
/// (doors closed — the editor's `InitialAngle 0`; furniture excluded because the Workbench
/// world places the furniture composition beside the building, not under it, so the oracle
/// never traced it) replayed against the 4000-pair door-inclusive oracle of T-090.11.3.
/// Measured 2026-09-04: 3983/4000 agree (re-blessed to 3998/4000 the same day under the
/// T-090.12.4 projectile layer policy, which drops the shell's `Building` physics mesh in
/// favour of its `FireView` fire geometry — 15 of the 17 phantoms were that mesh); the
/// shell alone scored 3965 with 20
/// model-clear/engine-blocked pairs — every one of them a closed door leaf, all recovered
/// here (0 left); the 17 model-blocked/engine-clear pairs are 15 the shell already had on
/// this larger oracle (roof ridge / eave skims at y ≈ 8.3 m and rays starting inside a
/// collider, which `TraceMove` ignores) plus 2 instance-owned ones, both window frames
/// (`Socket_Win_110x142_005` with the observer inside its collider at t = 0, and the
/// interior `socket_win_130x142_003` at t = 0.065). Re-bless deliberately, old → new in
/// the commit message.
#[test]
fn farmhouse_compound_door_parity_is_pinned() {
    let root = crate::repository_paths::test_repo_root();
    let buildings = terrain_dir(&root, "everon").join("prefabs/buildings");
    let shell_bytes = fs::read(buildings.join("FarmHouse_E_1L01_Wood.bvh")).expect("shell");
    let sc = BvhSidecar::parse(&shell_bytes).expect("shell parses");
    let shell = Arc::new(BvhSidecar {
        verts: sc.verts.clone(),
        tris: sc.tris.clone(),
        bvh: Bvh::build(&sc.verts, &sc.tris),
        kinds: sc.kinds.clone(),
    });
    let instances = buildings.join("FarmHouse_E_1L01_Wood.instances.json");
    assert_eq!(
        fs::read(&instances).expect("instances"),
        fs::read(fixture("FarmHouse_E_1L01_Wood.instances.golden.json")).expect("golden"),
        "map-assets instances diverged from the golden fixture — re-emit and re-bless both"
    );
    let (c, kept, dropped) =
        load_compound(&instances, shell, &["furniture".to_string()], false).expect("compound");
    // T-090.12.4 — 132 → 120: the projectile layer policy left the twelve `LightSwitch_02`
    // props (Prop preset, no fire geometry) out of the instance set.
    assert_eq!((kept, dropped), (120, 49));
    assert_eq!(c.doors().count(), 7);
    assert!(c.doors().all(|d| d.state == DoorState::Closed));

    let replay = |name: &str| -> (usize, usize, usize, usize) {
        let oracle: ParityFile =
            serde_json::from_str(&fs::read_to_string(fixture(name)).expect("oracle"))
                .expect("parse oracle");
        let (mut agree, mut missed, mut phantom) = (0usize, 0usize, 0usize);
        for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
            let clear = !c.blocked([ox, oy, oz], [tx, ty, tz]);
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
        (4000, 3998, 0, 2),
        "door-inclusive parity drifted (T-090.12.4: 3998/4000, 0 missed blocks, 2 phantoms — \
         3983/4000 with the Building physics mesh in the shell before the layer policy)"
    );
    // The T-090.6 oracle (doors and glass excluded) is untouched by the instances.
    assert_eq!(
        replay("FarmHouse_E_1L01_Wood_parity.json"),
        (400, 400, 0, 0),
        "shell oracle drifted under the compound"
    );
}
