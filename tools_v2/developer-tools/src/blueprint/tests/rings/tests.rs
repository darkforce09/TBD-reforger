use super::*;

fn grid_from(rows: &[&str]) -> PlanGrid {
    // rows[iz] is a string over ix: '#' covered. Row 0 = iz 0.
    let nz = rows.len();
    let nx = rows[0].len();
    let mut g = PlanGrid::new(nx, nz);
    for (iz, row) in rows.iter().enumerate() {
        for (ix, c) in row.chars().enumerate() {
            if c == '#' {
                g.set(ix, iz, true);
            }
        }
    }
    g
}

fn lattice(ring: &[[f64; 2]], cell: f64) -> Vec<(i64, i64)> {
    ring.iter()
        .map(|p| ((p[0] / cell).round() as i64, (p[1] / cell).round() as i64))
        .collect()
}

#[test]
fn full_rect_is_one_ccw_ring_of_four() {
    let t = trace(&grid_from(&["####", "####", "####"]), 0.1, 0.0);
    assert_eq!(t.pieces.len(), 1);
    assert_eq!(t.dropped, 0);
    let outer = lattice(&t.pieces[0].outer, 0.1);
    assert_eq!(outer, vec![(0, 0), (4, 0), (4, 3), (0, 3)]);
    assert!(t.pieces[0].holes.is_empty());
    assert!((t.pieces[0].area_m2 - 0.12).abs() < 1e-9);
}

#[test]
fn l_shape_traces_six_vertices() {
    // 4 wide × 2 tall base, 2 wide × 2 tall tower on top-left.
    let t = trace(&grid_from(&["####", "####", "##..", "##.."]), 0.1, 0.0);
    assert_eq!(t.pieces.len(), 1);
    let outer = lattice(&t.pieces[0].outer, 0.1);
    assert_eq!(outer, vec![(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)]);
}

#[test]
fn donut_has_one_cw_hole() {
    let t = trace(
        &grid_from(&["#####", "#####", "##.##", "#####", "#####"]),
        0.1,
        0.0,
    );
    assert_eq!(t.pieces.len(), 1);
    assert_eq!(t.pieces[0].holes.len(), 1);
    let hole = lattice(&t.pieces[0].holes[0], 0.1);
    // CW (negative shoelace), canonical start at lexicographic min (2,2).
    assert_eq!(hole, vec![(2, 2), (2, 3), (3, 3), (3, 2)]);
    let a2: i64 = shoelace_x2(&hole);
    assert!(a2 < 0, "hole must wind CW, area×2 = {a2}");
}

#[test]
fn disconnected_pieces_become_two_polygons() {
    let t = trace(&grid_from(&["##..##", "##..##"]), 0.1, 0.0);
    assert_eq!(t.pieces.len(), 2);
    assert!(t.pieces.iter().all(|p| p.holes.is_empty()));
    let (fp, polys) = t.contract();
    assert_eq!(polys.len(), 2);
    // Largest-first tie: equal areas — deterministic order by first vertex.
    assert_eq!(lattice(&fp, 0.1)[0], (0, 0));
}

#[test]
fn single_cell_is_a_four_vertex_ring() {
    let t = trace(&grid_from(&["#"]), 0.1, 0.0);
    assert_eq!(t.pieces.len(), 1);
    assert_eq!(t.pieces[0].outer.len(), 4);
}

#[test]
fn diagonal_touch_stays_two_separate_rings() {
    // Two cells sharing only the corner (1,1) — left-turn rule must NOT fuse them.
    let t = trace(&grid_from(&["#.", ".#"]), 0.1, 0.0);
    assert_eq!(t.pieces.len(), 2);
    assert!(t.pieces.iter().all(|p| p.outer.len() == 4));
}

#[test]
fn min_area_drops_noise_rings_and_counts_them() {
    // 3×3 block plus an isolated single cell (0.01 m² at 0.1 cell).
    let t = trace(&grid_from(&["###..", "###..", "###.#"]), 0.1, 0.02);
    assert_eq!(t.pieces.len(), 1);
    assert_eq!(t.dropped, 1);
}

#[test]
fn trace_is_deterministic() {
    let g = grid_from(&["#####", "#.###", "###.#", "#####"]);
    let a = trace(&g, 0.1, 0.0);
    let b = trace(&g, 0.1, 0.0);
    let ser = |t: &TracedRings| {
        t.pieces
            .iter()
            .map(|p| (p.outer.clone(), p.holes.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(ser(&a), ser(&b));
}
