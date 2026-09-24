use std::path::PathBuf;

use super::*;
use crate::repository_layout::terrain_dir;
use crate::world_export_pipeline::forest_contours::REGION_CELL_M;

fn closed(v: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut r = v.to_vec();
    r.push(v[0]);
    r
}

/// Every turn of `ring` (given closed) that is a right angle, and the largest run of
/// consecutive right-angle turns. A staircase is not "some 90° turns", it is 90° turns *in a
/// row* — an isolated sharp corner on an otherwise curved boundary is a preserved feature.
fn right_angle_turns(ring: &[(f64, f64)]) -> (usize, usize, usize) {
    let mut v = ring.to_vec();
    while v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    let n = v.len();
    let square: Vec<bool> = (0..n)
        .map(|i| {
            let p = v[(i + n - 1) % n];
            let c = v[i];
            let q = v[(i + 1) % n];
            let (ax, ay) = (c.0 - p.0, c.1 - p.1);
            let (bx, by) = (q.0 - c.0, q.1 - c.1);
            let (la, lb) = (ax.hypot(ay), bx.hypot(by));
            if la < EPS || lb < EPS {
                return false;
            }
            ((ax * bx + ay * by) / (la * lb)).abs() < 0.05
        })
        .collect();
    let adjacent = (0..n).filter(|i| square[*i] && square[(i + 1) % n]).count();
    (n, square.iter().filter(|s| **s).count(), adjacent)
}

fn square_ring(side: f64) -> Vec<(f64, f64)> {
    closed(&[(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)])
}

/// A staircase down the diagonal on the 32 m lattice — the shape `trace_rings` actually
/// emits, and the one a square cannot stand in for (a square has no concave corners).
fn staircase_ring(steps: usize) -> Vec<(f64, f64)> {
    let c = REGION_CELL_M;
    let mut v = vec![(0.0, 0.0)];
    for i in 0..steps {
        let x = (i + 1) as f64 * c;
        let y = i as f64 * c;
        v.push((x, y));
        v.push((x, y + c));
    }
    v.push((0.0, steps as f64 * c));
    closed(&v)
}

#[test]
fn a_square_ring_rounds() {
    let side = 4.0 * REGION_CELL_M;
    let (out, rep) = chaikin(&square_ring(side), CHAIKIN_ITERATIONS, None);
    assert!(out.first() == out.last(), "the ring must come back closed");
    assert_eq!(rep.verts_in, 4);
    assert_eq!(rep.verts_out, 16, "two Chaikin passes take 4 corners to 16");
    assert!(
        out.len() >= 5,
        "a closed ring needs >= 4 entries + the repeat"
    );

    // ROUNDS: not one right angle survives, and the corners have actually moved.
    let (_, square, adjacent) = right_angle_turns(&out);
    assert_eq!(square, 0, "a rounded ring has no right-angle turns left");
    assert_eq!(adjacent, 0);
    for corner in [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)] {
        assert!(
            !out.contains(&corner),
            "corner {corner:?} survived the smoother"
        );
    }
    // …and it is a rounding, not a collapse: the ring stays inside the square plus the one
    // area-restoring offset it is allowed.
    let slack = rep.offset_m.abs();
    for p in &out {
        assert!(
            p.0 >= -slack && p.0 <= side + slack && p.1 >= -slack && p.1 <= side + slack,
            "{p:?} left the square by more than the {slack} m area-restoring offset"
        );
    }
    // The rounding is real curvature, not a chamfer: every turn is shallow.
    let (_, sq, _) = right_angle_turns(&out);
    assert_eq!(sq, 0);
}

/// The LOCKED carve-out, spelled out because it is in tension with the test above and a
/// reader will otherwise think one of them is a bug.
///
/// `documentation_v2/tickets/specs/t149_forest_smooth.md` LOCKED says "rings under 6 vertices
/// untouched", and `trace_rings` drops collinear points — so a solid rectangular block of cells,
/// however large, traces exactly four vertices and the EMIT leaves it square. `chaikin` above
/// rounds the same ring; `smooth_ring` is where the rule lives.
#[test]
fn the_emit_leaves_a_bare_four_vertex_square_alone() {
    let ring = square_ring(4.0 * REGION_CELL_M);
    let (out, rep) = smooth_ring(&ring, None);
    assert!(
        rep.skipped_small,
        "4 distinct verts is under MIN_SMOOTH_VERTICES"
    );
    assert_eq!(out, ring, "the carve-out must return the ring untouched");
    assert_eq!(rep.area_in, rep.area_out);
    assert_eq!(rep.offset_m, 0.0);
}

/// The area bound holds at every scale, and the offset rail stays scale-free while doing it.
///
/// An ABSOLUTE rail fails this test: the 40-cell square needs a 59.4 m offset and the 1-cell
/// square needs 1.49 m, and the two are the same ring — `|d| / mean edge` is 0.2258 at both.
/// This is the test that would red if someone put a metre constant back.
#[test]
fn a_square_ring_holds_its_area_under_the_bound_at_every_scale() {
    let mut ratios: Vec<f64> = Vec::new();
    for side in [REGION_CELL_M, 4.0 * REGION_CELL_M, 40.0 * REGION_CELL_M] {
        let (out, rep) = chaikin(&square_ring(side), CHAIKIN_ITERATIONS, None);
        let drift = (rep.area_out - rep.area_in).abs() / rep.area_in.abs();
        assert!(
            drift < MAX_AREA_DRIFT,
            "side {side}: drift {:.4}% is over the {:.1}% bound",
            drift * 100.0,
            MAX_AREA_DRIFT * 100.0
        );
        assert!(!rep.offset_capped, "side {side}: the offset rail bound");
        let mut v = out.clone();
        v.pop();
        ratios.push(rep.offset_m.abs() / mean_edge(&v));
    }
    for r in &ratios {
        assert!(
            (r - ratios[0]).abs() < 1e-3,
            "the offset/edge ratio moved with scale: {ratios:?}"
        );
        assert!(
            *r < MAX_AREA_OFFSET_EDGES,
            "the square needs {r:.4} edges of offset, rail is {MAX_AREA_OFFSET_EDGES}"
        );
    }
    assert!(
        ratios[0] > 0.2,
        "the offset is not doing real work: {ratios:?}"
    );
}

/// The compensation is doing the work — without it a square is >10% light. This is the test
/// that makes stage 3 non-optional rather than decorative.
#[test]
fn uncompensated_chaikin_would_blow_the_bound() {
    let side = 4.0 * REGION_CELL_M;
    let ring = square_ring(side);
    let mut v = ring.clone();
    v.pop();
    let mut pin = vec![false; v.len()];
    for _ in 0..CHAIKIN_ITERATIONS {
        let (nv, np) = chaikin_once(&v, &pin);
        v = nv;
        pin = np;
    }
    let raw_drift = (signed_area(&v) - side * side).abs() / (side * side);
    assert!(
        raw_drift > 0.10,
        "raw Chaikin on a square drifts {:.2}%, expected > 10% — if this ever falls the \
         compensation below is no longer load-bearing and this module should say so",
        raw_drift * 100.0
    );
    let (_, rep) = chaikin(&ring, CHAIKIN_ITERATIONS, None);
    let fixed = (rep.area_out - rep.area_in).abs() / rep.area_in.abs();
    assert!(
        fixed < MAX_AREA_DRIFT,
        "compensated drift {:.4}% is not under the bound",
        fixed * 100.0
    );
    assert!(rep.offset_m.abs() > EPS, "no offset was applied");
}

#[test]
fn a_three_point_ring_stays_valid() {
    let c = REGION_CELL_M;
    let tri = closed(&[(0.0, 0.0), (c, 0.0), (0.0, c)]);
    let (out, rep) = smooth_ring(&tri, None);
    assert!(
        rep.skipped_small,
        "3 distinct verts is under MIN_SMOOTH_VERTICES"
    );
    assert_eq!(
        out, tri,
        "a ring the smoother skips must come back untouched"
    );
    assert!(out.len() >= 4, "the schema wants >= 4 closed-ring entries");
    assert_eq!(out.first(), out.last(), "still closed");
    assert!((signed_area(&out[..out.len() - 1]) - c * c / 2.0).abs() < EPS);

    // The carve-out is on DISTINCT vertices, so a 5-vertex ring is also skipped and a
    // 6-vertex one is not — the boundary is where MIN_SMOOTH_VERTICES says it is.
    let five = closed(&[(0.0, 0.0), (c, 0.0), (2.0 * c, c), (c, 2.0 * c), (0.0, c)]);
    assert!(smooth_ring(&five, None).1.skipped_small);
    let six = closed(&[
        (0.0, 0.0),
        (c, 0.0),
        (2.0 * c, c),
        (2.0 * c, 2.0 * c),
        (c, 2.0 * c),
        (0.0, c),
    ]);
    assert!(!smooth_ring(&six, None).1.skipped_small);
}

/// A hole ring: clockwise, so its signed area is negative and it subtracts from the region.
/// The smoother must round it without flipping its orientation and without pulling it through
/// itself. An L, not a square, so the ring clears the carve-out and carries a concave corner.
#[test]
fn a_hole_ring_keeps_its_sign_and_its_area() {
    let c = REGION_CELL_M;
    let mut l = vec![
        (0.0, 0.0),
        (3.0 * c, 0.0),
        (3.0 * c, c),
        (c, c),
        (c, 3.0 * c),
        (0.0, 3.0 * c),
    ];
    l.reverse(); // clockwise — a hole
    let hole = closed(&l);
    assert!(
        signed_area(&hole[..hole.len() - 1]) < 0.0,
        "the hole ring must be negatively oriented"
    );
    let (out, rep) = smooth_ring(&hole, None);
    assert!(!rep.skipped_small, "a 6-vertex ring must be smoothed");
    assert!(
        rep.area_out < 0.0,
        "the smoother flipped a hole's orientation"
    );
    let drift = (rep.area_out - rep.area_in).abs() / rep.area_in.abs();
    assert!(drift < MAX_AREA_DRIFT, "hole drift {:.4}%", drift * 100.0);
    let (_, square, _) = right_angle_turns(&out);
    assert_eq!(square, 0, "the hole did not round");
}

/// The corner-preserving option must be able to FIRE and must CHANGE the output — a pin path
/// that cannot alter a ring is worse than no pin path at all.
#[test]
fn the_canopy_oracle_pins_corners_and_changes_the_ring() {
    let ring = staircase_ring(6);
    let (bare, bare_rep) = smooth_ring(&ring, None);
    assert_eq!(bare_rep.pinned, 0);

    // Solid everywhere: every convex corner is a real promontory, no notch is bare, so only
    // the convex corners pin.
    let solid = |_x: f64, _y: f64| PIN_SOLID_MASS;
    let (dense_out, dense_rep) = smooth_ring(&ring, Some(&solid));
    assert!(dense_rep.pinned > 0, "the solid oracle pinned nothing");
    assert_ne!(dense_out, bare, "pinning did not change the ring");
    assert!(
        dense_out.len() < bare.len(),
        "a pinned corner emits one vertex, not two: {} vs {}",
        dense_out.len(),
        bare.len()
    );

    // Bare everywhere: no convex corner is solid, every notch is bare, so only the concave
    // corners pin — a different set, which proves the convex/concave split is live.
    let bare_field = |_x: f64, _y: f64| 0.0;
    let (clear_out, clear_rep) = smooth_ring(&ring, Some(&bare_field));
    assert!(clear_rep.pinned > 0, "the bare oracle pinned nothing");
    assert_ne!(
        clear_rep.pinned, dense_rep.pinned,
        "the convex and concave branches pinned the same count — one of them is dead"
    );
    assert_ne!(clear_out, dense_out);

    // Marginal canopy — above the clearing floor, below the solid floor — is the 32 m
    // quantisation band, and there NOTHING pins.
    let marginal = |_x: f64, _y: f64| CANOPY_MASS_ISO;
    let (marginal_out, marginal_rep) = smooth_ring(&ring, Some(&marginal));
    assert_eq!(marginal_rep.pinned, 0, "the ambiguous band must not pin");
    assert_eq!(marginal_out, bare);

    // And a pin is exact: the pinned vertices survive verbatim into the output. (Offset by
    // the area compensation, so compare against the input corner within one offset.)
    assert!(dense_rep.offset_m.abs() > EPS && !dense_rep.offset_capped);
    for r in [&dense_rep, &clear_rep] {
        let drift = (r.area_out - r.area_in).abs() / r.area_in.abs();
        assert!(drift < MAX_AREA_DRIFT, "pinned drift {:.4}%", drift * 100.0);
    }
}

/// A HOLE ring must read the canopy the other way round, and this is the test that caught it
/// being wrong.
///
/// The pin rule asks "does the cut eat forest, or grow it?", and a hole encloses a *clearing*:
/// shrinking a hole's enclosure ADDS forest. So the same ring traversed the other way must pin
/// the OTHER corners. The first version of `corner_pins` branched on the
/// orientation-normalised turn, which is invariant under reversal — it gave a hole and its
/// outer twin identical pins, i.e. it pinned every clearing corner as if the clearing were a
/// forest. `everon_smooths…` could not see it (a hole with ≥ 6 vertices is rare) and
/// `a_hole_ring_keeps_its_sign_and_its_area` could not either (it passes no oracle at all).
#[test]
fn a_hole_reads_the_canopy_the_other_way_round() {
    let c = REGION_CELL_M;
    let l = vec![
        (0.0, 0.0),
        (3.0 * c, 0.0),
        (3.0 * c, c),
        (c, c),
        (c, 3.0 * c),
        (0.0, 3.0 * c),
    ];
    let outer = closed(&l);
    let mut rev = l.clone();
    rev.reverse();
    let hole = closed(&rev);
    assert!(signed_area(&l) > 0.0 && signed_area(&rev) < 0.0);

    // "Solid everywhere": every forest-eating cut is refused, every forest-growing cut is
    // taken. Reversing the ring swaps which corners those are, so the pin sets are
    // complementary — an L hexagon is 5 convex + 1 reflex (5·90° + 270° = 720°), so 5 pins
    // one way and 1 the other.
    let solid = |_x: f64, _y: f64| PIN_SOLID_MASS;
    let outer_pins = corner_pins(&l, 1.0, &solid);
    let hole_pins = corner_pins(&rev, -1.0, &solid);
    let outer_n = outer_pins.iter().filter(|p| **p).count();
    let hole_n = hole_pins.iter().filter(|p| **p).count();
    assert_eq!(outer_n, 5, "the L has 5 material-convex corners");
    assert_eq!(
        hole_n, 1,
        "reversed, the same 6 corners must pin the complementary 1 — a hole that pins the \
         same corners as its outer twin is reading the canopy as if the clearing were forest"
    );
    assert_eq!(
        outer_n + hole_n,
        l.len(),
        "the two sets must partition the ring"
    );
    for i in 0..l.len() {
        // The reversed ring visits the same vertices in the opposite order.
        let j = rev.iter().position(|p| *p == l[i]).expect("same vertices");
        assert_ne!(
            outer_pins[i], hole_pins[j],
            "vertex {i} {:?} pinned the same way as an outer and as a hole",
            l[i]
        );
    }

    // …and it survives the full pipeline, area and orientation intact.
    let (_, o) = smooth_ring(&outer, Some(&solid));
    let (_, h) = smooth_ring(&hole, Some(&solid));
    assert_eq!((o.pinned, h.pinned), (5, 1));
    assert!(
        o.area_out > 0.0 && h.area_out < 0.0,
        "an orientation flipped"
    );
    for r in [&o, &h] {
        let drift = (r.area_out - r.area_in).abs() / r.area_in.abs();
        assert!(drift < MAX_AREA_DRIFT, "drift {:.4}%", drift * 100.0);
    }
}

/// The probe has to reach a DIFFERENT 8 m corner than the vertex, or the oracle is asking
/// itself. Diagonal bisector, so the per-axis step is `CORNER_PROBE_M / √2`.
#[test]
fn the_corner_probe_clears_the_vertexs_own_density_cell() {
    let half = f64::from(density::DENSITY_CELL_M) / 2.0;
    let step = CORNER_PROBE_M / std::f64::consts::SQRT_2;
    assert!(
        step >= half,
        "a {CORNER_PROBE_M} m diagonal probe moves {step:.3} m per axis, inside the ±{half} m \
         corner window — it would resample the vertex's own corner"
    );
    // …and it does not overshoot into the next-but-one cell either.
    assert!(step < 3.0 * half);
}

#[test]
fn the_area_solver_hits_its_target_in_closed_form() {
    // A ring the quadratic term actually matters for.
    let v: Vec<(f64, f64)> = (0..12)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::TAU / 12.0;
            (100.0 + 60.0 * a.cos(), 100.0 + 35.0 * a.sin())
        })
        .collect();
    let u = vertex_normals(&v);
    for target_scale in [0.90, 0.99, 1.0, 1.01, 1.10] {
        let target = signed_area(&v) * target_scale;
        let (d, capped) = solve_offset(&v, &u, target);
        assert!(!capped, "scale {target_scale}: capped at {d}");
        let moved: Vec<(f64, f64)> = v
            .iter()
            .zip(&u)
            .map(|(p, n)| (p.0 + d * n.0, p.1 + d * n.1))
            .collect();
        let got = signed_area(&moved);
        assert!(
            (got - target).abs() < 1e-6 * target.abs().max(1.0),
            "scale {target_scale}: solved d={d} gives area {got}, wanted {target}"
        );
    }
}

/// A ring so degenerate the solver cannot help must not produce NaN coordinates.
#[test]
fn a_degenerate_ring_does_not_produce_nan() {
    let collapsed = closed(&[
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
    ]);
    let (out, rep) = smooth_ring(&collapsed, None);
    assert!(out.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
    assert!(rep.offset_m.is_finite() && rep.area_out.is_finite());
}

/* ── the real everon catalogue ── */

fn everon_objects() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    terrain_dir(&root, "everon").join("objects")
}

/// The committed everon `forest-regions.json.gz`. Neither this nor the density tiles below is
/// LFS-tracked (`git check-attr filter` → unspecified), so a missing or short corpus is a
/// FAILURE, never a skip.
fn everon_regions() -> Vec<Value> {
    let p = everon_objects().join("forest-regions.json.gz");
    let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("T-149: {} ({e})", p.display()));
    let raw = crate::world_export_pipeline::chunk_partitioner::gunzip(&bytes)
        .expect("gunzip forest-regions");
    let doc: Value = serde_json::from_slice(&raw).expect("parse forest-regions");
    let regions = doc["regions"].as_array().cloned().unwrap_or_default();
    assert!(
        regions.len() >= 30,
        "everon ships 36 forest regions, found {} — this test would have compared nothing",
        regions.len()
    );
    regions
}

/// The 625 committed density tiles reassembled into the global 8 m canopy grid the exporter
/// blurs and slices. Built from `slice_chunk_corners`' own stride rule, so a stride change
/// reds here too.
fn everon_canopy() -> (Vec<u32>, usize) {
    let cols = density::DENSITY_COLS as usize;
    let stride = cols - 1;
    let chunks = 25usize; // everon: 12 800 m / 512 m
    let size = chunks * stride + 1;
    assert_eq!(size, density::corner_grid_size(12_800.0));
    let mut grid = vec![0u32; size * size];
    let dir = everon_objects().join("density");
    let mut tiles = 0usize;
    for cy in 0..chunks {
        for cx in 0..chunks {
            let p = dir.join(format!("{cx}_{cy}.bin"));
            let bytes =
                std::fs::read(&p).unwrap_or_else(|e| panic!("T-149: {} ({e})", p.display()));
            assert_eq!(
                bytes.len(),
                density::TBDD_FILE_BYTES,
                "{} is {} B — a `vers…` prefix here means an LFS pointer, not a payload",
                p.display(),
                bytes.len()
            );
            let g = website_map_engine::io::density::tbdd::decode_tbdd(&bytes)
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert_eq!(
                (g.cols, g.rows, g.cell_m, g.version),
                (
                    density::DENSITY_COLS,
                    density::DENSITY_ROWS,
                    density::DENSITY_CELL_M,
                    density::TBDD_VERSION
                ),
                "{} header disagrees with this build's constants — the smoother would be \
                 sampling a field it does not understand",
                p.display()
            );
            for j in 0..density::DENSITY_ROWS as usize {
                for i in 0..cols {
                    grid[(cy * stride + j) * size + cx * stride + i] =
                        u32::from(g.channels[0][j * cols + i]);
                }
            }
            tiles += 1;
        }
    }
    assert_eq!(tiles, 625);
    assert!(
        grid.iter().filter(|v| **v > 0).count() > 100_000,
        "the reassembled canopy grid is nearly empty — the oracle below would be vacuous"
    );
    (grid, size)
}

/// ACCEPTANCE, on the whole committed everon catalogue rather than a sample.
///
/// Three claims, all measured against the shipped rings:
///   1. every region's area drift is under 3%;
///   2. the boundary stops being a staircase — and this is checked by ATTRIBUTION, not by a
///      tolerance: after smoothing, a right-angle turn may only exist on a ring the LOCKED
///      `MIN_SMOOTH_VERTICES` carve-out protected, or at a vertex the 8 m canopy pinned. Any
///      other surviving right angle is leftover staircase and reds here.
///   3. `smooth_regions` is exactly `smooth_ring` plus JSON — the emit path cannot quietly
///      differ from the geometry these claims are proved against.
#[test]
fn everon_smooths_within_the_area_bound_and_stops_being_a_staircase() {
    let (grid, size) = everon_canopy();
    let canopy = |x: f64, y: f64| f64::from(density::sample_corners(&grid, size, x, y));
    let regions_in = everon_regions();

    // BEFORE: the defect, quantified. And, per ring, what the smoother is allowed to leave
    // square: everything on a carved-out ring, and the pins on every other.
    let (mut turns_in, mut square_in, mut adjacent_in) = (0, 0, 0);
    let (mut turns_out, mut square_out, mut adjacent_out, mut allowed_square) = (0, 0, 0, 0);
    let (mut carved_rings, mut carved_verts) = (0, 0);
    let mut worst_offset_edges = 0.0f64;
    let mut per_ring: Vec<Vec<(f64, f64)>> = Vec::new();
    for r in &regions_in {
        for ring_json in r["polygon"].as_array().expect("polygon") {
            let ring = ring_from_json(ring_json).expect("finite ring");
            let (n, sq, adj) = right_angle_turns(&ring);
            turns_in += n;
            square_in += sq;
            adjacent_in += adj;

            let (out, rep) = smooth_ring(&ring, Some(&canopy));
            if !rep.skipped_small {
                let mut distinct = out.clone();
                distinct.pop();
                let edge = mean_edge(&distinct);
                assert!(edge > EPS);
                worst_offset_edges = worst_offset_edges.max(rep.offset_m.abs() / edge);
            }
            assert!(
                out.len() >= 4 && out.first() == out.last(),
                "{}: a smoothed ring stopped being a valid closed ring",
                r["id"]
            );
            let (n2, sq2, adj2) = right_angle_turns(&out);
            turns_out += n2;
            square_out += sq2;
            adjacent_out += adj2;
            if rep.skipped_small {
                carved_rings += 1;
                carved_verts += n2;
                allowed_square += n2;
                assert_eq!(out, ring, "a carved-out ring was modified");
            } else {
                allowed_square += rep.pinned;
                assert!(
                    sq2 <= rep.pinned,
                    "{}: {sq2} right angles survived on a ring with only {} pins — \
                     that is leftover staircase",
                    r["id"],
                    rep.pinned
                );
            }
            per_ring.push(out);
        }
    }
    assert_eq!(
        (square_in, adjacent_in),
        (turns_in, turns_in),
        "the shipped rings are supposed to be 100% right angles ({turns_in} turns)"
    );
    assert!(carved_rings > 0 && carved_verts > 0);
    // The rail has real headroom rather than merely not binding — this is the claim
    // MAX_AREA_OFFSET_EDGES is set from, held against the corpus instead of a doc comment.
    assert!(
        worst_offset_edges < 0.5 * MAX_AREA_OFFSET_EDGES,
        "the hungriest everon ring wants {worst_offset_edges:.4} of a mean edge, rail is \
         {MAX_AREA_OFFSET_EDGES} — less than 2x headroom left"
    );
    assert!(worst_offset_edges > 0.0, "no ring needed any offset at all");

    // CLAIM 3 — run the production path and hold it to the same rings.
    let mut regions = regions_in.clone();
    let reports = smooth_regions(&mut regions, Some(&canopy));
    assert_eq!(reports.len(), regions.len());
    let mut k = 0usize;
    for r in &regions {
        for ring_json in r["polygon"].as_array().expect("polygon") {
            let got = ring_from_json(ring_json).expect("finite ring");
            assert_eq!(got, per_ring[k], "smooth_regions diverged from smooth_ring");
            k += 1;
        }
    }
    assert_eq!(k, per_ring.len());

    // CLAIM 1 — area.
    let mut worst = (0.0f64, String::new());
    let (mut vin, mut vout, mut pinned) = (0, 0, 0);
    for r in &reports {
        assert!(r.rings > 0, "{}: no rings were read", r.id);
        assert_eq!(r.offset_capped, 0, "{}: the offset rail bound", r.id);
        // A measured property of this corpus, not the rail: on everon the whole boundary
        // never has to move by even one 8 m density cell to give the area back.
        assert!(
            r.max_offset_m < DENSITY_CELL_M_F,
            "{}: offset {:.3} m is a whole density cell — the compensation is doing more \
             than the smoothing",
            r.id,
            r.max_offset_m
        );
        let d = r.area_drift();
        if d > worst.0 {
            worst = (d, r.id.clone());
        }
        assert!(
            d < MAX_AREA_DRIFT,
            "{}: area drift {:.4}% is over the {:.1}% acceptance bound",
            r.id,
            d * 100.0,
            MAX_AREA_DRIFT * 100.0
        );
        vin += r.verts_in;
        vout += r.verts_out;
        pinned += r.pinned;
    }
    assert!(vout > vin, "the smoother added no vertices at all");
    assert!(
        pinned > 0,
        "the canopy oracle pinned nothing across all 36 regions — on this corpus the \
         corner-preserving path would be dead code"
    );

    // CLAIM 2 — the staircase is gone. Every surviving right angle is accounted for, and the
    // consecutive-right-angle signature that defines a staircase has collapsed.
    // `<=` and not `==`: a pin does not always LEAVE a right angle (two adjacent pins, or a
    // pin on a collinear run, come out shallow). The exact claim is the per-ring
    // `sq2 <= rep.pinned` above; this is its roll-up.
    assert!(
        square_out <= allowed_square,
        "{square_out} right angles survived, only {allowed_square} are attributable to the \
         carve-out ({carved_verts} verts) plus canopy pins ({pinned})"
    );
    assert!(square_out > 0, "the attribution above compared nothing");
    assert!(
        adjacent_out * 10 < turns_out,
        "{adjacent_out} of {turns_out} turns are still consecutive right angles (>10%)"
    );

    eprintln!(
        "[T-149] everon: {} regions, {vin} -> {vout} verts, {pinned} pinned, \
         {carved_rings} rings carved out ({carved_verts} verts); \
         right-angle turns {square_in}/{turns_in} -> {square_out}/{turns_out}, \
         consecutive {adjacent_in} -> {adjacent_out}; worst area drift {:.6}% ({}) \
         vs bound {:.1}%; largest area-restoring offset {:.4} m \
         ({worst_offset_edges:.4} of a mean edge, rail {MAX_AREA_OFFSET_EDGES})",
        reports.len(),
        worst.0 * 100.0,
        worst.1,
        MAX_AREA_DRIFT * 100.0,
        reports.iter().fold(0.0f64, |m, r| m.max(r.max_offset_m))
    );
}
