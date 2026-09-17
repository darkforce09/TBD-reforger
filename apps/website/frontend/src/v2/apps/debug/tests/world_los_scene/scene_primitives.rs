use super::*;

fn fp(pid: u16, kind: &str, x: f64, proxy: bool) -> Footprint {
    Footprint {
        pid,
        kind: kind.into(),
        min: [x, 0.0],
        max: [x + 2.0, 2.0],
        proxy,
    }
}

fn hit(t: f64, kind: LosHitKind, c: f64) -> LosHit {
    LosHit {
        t,
        pos: [0.0; 3],
        kind,
        id: "x".into(),
        concealment: c,
    }
}

#[test]
fn footprints_route_by_kind_and_proxies_are_amber_outlines() {
    let l = build_bench_lanes(
        &[
            fp(1, "building", 0.0, false),
            fp(2, "tree", 10.0, false),
            fp(3, "rock", 20.0, false),
            fp(4, "prop", 30.0, false),
            fp(5, "vehicle", 40.0, false),
            fp(6, "building", 50.0, true),
        ],
        &[[[0.0, 0.0], [2.0, 0.0]]],
    );
    assert_eq!(l.slabs_idx.len(), 6, "one building quad");
    assert_eq!(l.vegetation_idx.len(), 12, "tree + rock quads");
    assert_eq!(l.furniture_idx.len(), 12, "prop + vehicle quads");
    assert_eq!(l.furniture_count, 2);
    assert_eq!(l.tree_count, 1);
    assert_eq!(l.vegetation_outline_count, 4, "the tree's rim");
    assert_eq!(l.portals_outline_count, 4, "the proxy's amber outline");
    assert_eq!(l.wall_count, 1, "one section cut strip");
    assert_eq!(l.walls.len() % 6, 0);
    assert_eq!(kind_color("water"), COL_WATER);
    assert_eq!(kind_color("anything"), COL_PROP);
}

#[test]
fn ray_spans_follow_the_crossings() {
    let (packed, n) = ray_strip([0.0, 0.0], [100.0, 0.0], &[], true, false);
    assert_eq!(n, 1);
    assert_eq!(packed[2..6], RAY_CLEAR);
    // Blocked halfway: green then red, nothing after the terminal hit is recoloured.
    let (packed, n) = ray_strip(
        [0.0, 0.0],
        [100.0, 0.0],
        &[
            hit(0.5, LosHitKind::Solid, 1.0),
            hit(0.8, LosHitKind::Glass, 0.05),
        ],
        false,
        false,
    );
    assert_eq!(n, 2);
    let stride = packed.len() / 2;
    assert_eq!(packed[2..6], RAY_CLEAR);
    assert_eq!(packed[stride + 2..stride + 6], RAY_BLOCKED);
    // Glass then foliage, clear: three spans, cyan then yellow-green.
    let (packed, n) = ray_strip(
        [0.0, 0.0],
        [100.0, 0.0],
        &[
            hit(0.2, LosHitKind::Glass, 0.05),
            hit(0.6, LosHitKind::Foliage, 0.3),
        ],
        true,
        false,
    );
    assert_eq!(n, 3);
    let stride = packed.len() / 3;
    assert_eq!(packed[stride + 2..stride + 6], RAY_GLASS);
    assert_eq!(packed[2 * stride + 2..2 * stride + 6], RAY_FOLIAGE);
    // Provisional: the tail is amber.
    let (packed, _) = ray_strip([0.0, 0.0], [100.0, 0.0], &[], false, true);
    assert_eq!(packed[2..6], RAY_PROVISIONAL);
}
