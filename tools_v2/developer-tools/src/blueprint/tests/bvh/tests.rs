use super::*;
use crate::blueprint::tests::fixture;

/// The engine-free parity pin: the committed sidecar replayed against the committed
/// 400-pair Workbench oracle — CI re-proves the 3D lane without the (unshippable)
/// .xob. Numbers measured live 2026-09-01; re-bless deliberately, old→new in the
/// commit message.
#[test]
fn farmhouse_bvh_sidecar_parity_is_pinned() {
    let golden =
        fs::read(fixture("FarmHouse_E_1L01_Wood.bvh.golden")).expect("golden sidecar fixture");
    // The shipping sidecar and the test golden are the same bytes, forever.
    let shipping = crate::repository_paths::test_repo_root()
        .join("packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.bvh");
    assert_eq!(
        golden,
        fs::read(&shipping).expect("shipping sidecar"),
        "map-assets sidecar diverged from the golden fixture — re-emit and re-bless both"
    );
    let sc = BvhSidecar::parse(&golden).expect("golden sidecar parses");
    assert_eq!(
        (sc.verts.len(), sc.tris.len(), sc.bvh.node_count()),
        (3170, 2883, 1125),
        "sidecar shape drifted"
    );

    let oracle: ParityFile = serde_json::from_str(
        &fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_parity.json")).expect("oracle"),
    )
    .expect("parse oracle");
    assert_eq!(oracle.pairs.len(), 400);
    let mut agree = 0usize;
    let mut phantom = 0usize;
    for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
        let clear = sc
            .bvh
            .any_hit(&sc.verts, &sc.tris, [ox, oy, oz], [tx, ty, tz], 0.0, 1.0)
            .is_none();
        if clear == engine_clear {
            agree += 1;
        } else if !clear {
            phantom += 1;
        }
    }
    assert_eq!(agree, 400, "sidecar parity drifted (was 100.0%)");
    assert_eq!(phantom, 0, "phantom geometry blocks rays");
}
