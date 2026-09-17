use super::*;
use serde_json::json;
use website_map_engine::data::store::operations::tactical_graphics::mint_graphic_id;

fn env(rows: Value) -> Value {
    json!({"weather": "clear", "tacticalGraphics": rows})
}

fn one_of_each() -> Value {
    env(json!([
        {"id": "pl", "kind": "phase_line", "points": [[0.0, 0.0], [100.0, 0.0]],
         "label": "PL BLUE"},
        {"id": "bd", "kind": "boundary", "points": [[0.0, 500.0], [100.0, 500.0], [200.0, 600.0]]},
        {"id": "ax", "kind": "axis_of_advance", "points": [[0.0, 1000.0], [1000.0, 1000.0]]},
        {"id": "ar", "kind": "curved_arrow",
         "points": [[0.0, 2000.0], [500.0, 2200.0], [1000.0, 2000.0]]}
    ]))
}

#[test]
fn all_four_kinds_parse_and_keep_their_authored_vertices() {
    let gs = tactical_graphics_from_env(&one_of_each());
    assert_eq!(gs.len(), 4);
    assert_eq!(gs[0].id, "pl");
    assert_eq!(gs[0].kind, "phase_line");
    assert_eq!(gs[0].label, "PL BLUE");
    assert_eq!(gs[0].points, vec![[0.0, 0.0], [100.0, 0.0]]);
    assert_eq!(gs[1].points.len(), 3);
    assert_eq!(gs[3].kind, "curved_arrow");
}

#[test]
fn every_kind_produces_drawable_segments_and_the_lane_packs_them() {
    let gs = tactical_graphics_from_env(&one_of_each());
    for g in &gs {
        assert!(
            !graphic_segments(g).is_empty(),
            "{} must draw something",
            g.kind
        );
    }
    let verts = tactical_lane_verts(&gs, None);
    assert_eq!(verts.len() % 12, 0, "6 floats/vertex, 2 vertices/segment");
    let total: usize = gs.iter().map(|g| graphic_segments(g).len()).sum();
    assert_eq!(lane_segment_count(&verts) as usize, total);
    assert!(tactical_lane_verts(&[], None).is_empty());
}

/// The two directed kinds get barbs and the two undirected ones do not — the claim the module
/// header makes in prose, asserted in segment counts.
#[test]
fn only_the_directed_kinds_carry_an_arrowhead_and_only_a_boundary_carries_ticks() {
    assert!(kind_has_arrowhead("axis_of_advance"));
    assert!(kind_has_arrowhead("curved_arrow"));
    assert!(!kind_has_arrowhead("phase_line"));
    assert!(!kind_has_arrowhead("boundary"));

    let gs = tactical_graphics_from_env(&one_of_each());
    let by = |id: &str| gs.iter().find(|g| g.id == id).expect("row").clone();

    // A 2-point phase line is exactly one span, no ornament.
    assert_eq!(graphic_segments(&by("pl")).len(), 1);
    // A 2-point axis is one span plus two barbs.
    assert_eq!(graphic_segments(&by("ax")).len(), 3);
    // A 3-point boundary is two spans plus three ticks, and no barbs.
    assert_eq!(graphic_segments(&by("bd")).len(), 5);
    // A 3-point curve tessellates to 2 spans x CURVE_SAMPLES_PER_SPAN, plus two barbs.
    assert_eq!(
        graphic_segments(&by("ar")).len(),
        2 * CURVE_SAMPLES_PER_SPAN + 2
    );
}

#[test]
fn the_curve_passes_through_its_authored_vertices_and_bows_off_the_chord() {
    let pts = [[0.0, 0.0], [500.0, 200.0], [1000.0, 0.0]];
    let curve = catmull_rom(&pts, CURVE_SAMPLES_PER_SPAN);
    assert_eq!(curve.len(), 2 * CURVE_SAMPLES_PER_SPAN + 1);
    assert_eq!(curve[0], pts[0]);
    assert_eq!(*curve.last().expect("non-empty"), pts[2]);
    assert!(
        curve
            .iter()
            .any(|p| (p[0] - pts[1][0]).abs() < 1.0 && (p[1] - pts[1][1]).abs() < 1.0),
        "the spline must pass through the interior authored vertex"
    );
    // A bow, not a chord: the chord from (0,0) to (1000,0) is y = 0 everywhere.
    assert!(
        curve.iter().any(|p| p[1] > 20.0),
        "a Catmull-Rom through a raised middle vertex must leave the chord: {curve:?}"
    );
    // Every sample is finite — the repeated-point epsilon guard, driven directly.
    let doubled = [[0.0, 0.0], [0.0, 0.0], [10.0, 10.0], [10.0, 10.0]];
    assert!(
        catmull_rom(&doubled, 4)
            .iter()
            .all(|p| p[0].is_finite() && p[1].is_finite()),
        "repeated vertices must not produce NaN"
    );
    // Too few points is returned unchanged rather than invented into a curve.
    assert_eq!(catmull_rom(&pts[..2], 8), pts[..2].to_vec());
}

#[test]
fn the_line_pick_is_point_to_segment_and_nearest_wins() {
    let gs = tactical_graphics_from_env(&env(json!([
        {"id": "a", "kind": "phase_line", "points": [[0.0, 0.0], [100.0, 0.0]]},
        {"id": "b", "kind": "phase_line", "points": [[0.0, 20.0], [100.0, 20.0]]}
    ])));
    assert_eq!(
        pick_tactical_graphic(&gs, 50.0, 3.0, 6.0).as_deref(),
        Some("a")
    );
    assert_eq!(
        pick_tactical_graphic(&gs, 50.0, 17.0, 6.0).as_deref(),
        Some("b")
    );
    // Nearest wins where both are in range.
    assert_eq!(
        pick_tactical_graphic(&gs, 50.0, 12.0, 30.0).as_deref(),
        Some("b")
    );
    // Off the END of the span but on its infinite extension: a miss, not a hit on nothing.
    assert_eq!(pick_tactical_graphic(&gs, 200.0, 0.0, 6.0), None);
    assert_eq!(pick_tactical_graphic(&gs, 50.0, 60.0, 6.0), None);
    assert_eq!(pick_tactical_graphic(&[], 0.0, 0.0, 1e6), None);
}

/// A click on the VISIBLE curve finds the graphic even where the chord between the authored
/// vertices is far away — the reason the pick runs over `graphic_segments` and not `points`.
#[test]
fn the_pick_follows_the_drawn_curve_not_the_authored_chord() {
    let gs = tactical_graphics_from_env(&env(json!([
        {"id": "ar", "kind": "curved_arrow",
         "points": [[0.0, 0.0], [500.0, 300.0], [1000.0, 0.0]]}
    ])));
    let curve = drawn_polyline(&gs[0]);
    assert_eq!(
        curve.len(),
        2 * CURVE_SAMPLES_PER_SPAN + 1,
        "the curve must be TESSELLATED, not the three authored vertices"
    );
    // The probe is the drawn point FURTHEST from both authored chords. Taking the apex of the
    // curve instead would be a probe derived from the function under test in a way that
    // follows it anywhere: a tessellation that silently collapsed to the chords would still
    // put its highest point on a chord and still be "hit".
    let clearance = |p: [f64; 2]| {
        dist_to_segment(p[0], p[1], [0.0, 0.0], [500.0, 300.0]).min(dist_to_segment(
            p[0],
            p[1],
            [500.0, 300.0],
            [1000.0, 0.0],
        ))
    };
    let off_chord = curve
        .iter()
        .copied()
        .max_by(|a, b| clearance(*a).total_cmp(&clearance(*b)))
        .expect("curve has samples");
    assert!(
        clearance(off_chord) > 20.0,
        "the spline must leave the authored chords by more than the pick tolerance, else \
         this test cannot tell a curve from a chord: {off_chord:?} clear {}",
        clearance(off_chord)
    );
    assert_eq!(
        pick_tactical_graphic(&gs, off_chord[0], off_chord[1], 5.0).as_deref(),
        Some("ar"),
        "a click on the drawn curve, off the authored chords, must hit"
    );
    // ...and the SAME click must MISS a straight kind through the identical vertices. That is
    // what proves the hit above came from the tessellation rather than from a loose tolerance.
    let straight = tactical_graphics_from_env(&env(json!([
        {"id": "ax", "kind": "axis_of_advance",
         "points": [[0.0, 0.0], [500.0, 300.0], [1000.0, 0.0]]}
    ])));
    assert_eq!(
        pick_tactical_graphic(&straight, off_chord[0], off_chord[1], 5.0),
        None,
        "the straight kind draws the chords, so the off-chord probe must miss it"
    );
}

#[test]
fn the_vertex_pick_returns_authored_indices_only() {
    let gs = tactical_graphics_from_env(&one_of_each());
    assert_eq!(
        pick_tactical_vertex(&gs, 100.0, 0.0, 5.0),
        Some(("pl".to_string(), 1))
    );
    assert_eq!(
        pick_tactical_vertex(&gs, 200.0, 600.0, 5.0),
        Some(("bd".to_string(), 2))
    );
    assert_eq!(pick_tactical_vertex(&gs, 50.0, 0.0, 5.0), None, "mid-span");
    // A curve's tessellated samples are NOT vertices: the apex of `ar` is a drawn point but
    // has no authored index, so it must not be draggable.
    let apex = drawn_polyline(&gs[3])
        .into_iter()
        .max_by(|a, b| a[1].total_cmp(&b[1]))
        .expect("samples");
    let hit = pick_tactical_vertex(&gs, apex[0], apex[1], 5.0);
    assert!(
        hit.is_none() || hit.as_ref().is_some_and(|(_, i)| *i == 1),
        "only an authored index may come back: {hit:?}"
    );
}

#[test]
fn selection_tints_exactly_one_graphic_and_style_overrides_the_kind_default() {
    let gs = tactical_graphics_from_env(&env(json!([
        {"id": "a", "kind": "phase_line", "points": [[0.0, 0.0], [100.0, 0.0]]},
        {"id": "b", "kind": "phase_line", "points": [[0.0, 20.0], [100.0, 20.0]],
         "style": {"color": "#ff0000", "alpha": 0.5}}
    ])));
    assert_eq!(gs[0].rgba, default_kind_rgba("phase_line"));
    assert_eq!(gs[1].rgba, [1.0, 0.0, 0.0, 0.5]);

    let v = tactical_lane_verts(&gs, Some("b"));
    // Segment 0 is `a` (kind default), segment 1 is `b` (selected amber).
    assert_eq!(&v[2..6], &default_kind_rgba("phase_line"));
    assert_eq!(&v[14..18], &TG_SELECTED_RGBA);
    // A selection naming nothing tints nothing — the stale-selection case.
    let none = tactical_lane_verts(&gs, Some("gone"));
    assert_eq!(&none[14..18], &gs[1].rgba);
}

#[test]
fn a_malformed_row_is_skipped_rather_than_drawn_wrong() {
    let gs = tactical_graphics_from_env(&env(json!([
        {"id": "", "kind": "phase_line", "points": [[0.0, 0.0], [1.0, 1.0]]},
        {"id": "no-kind", "points": [[0.0, 0.0], [1.0, 1.0]]},
        {"id": "short", "kind": "phase_line", "points": [[0.0, 0.0]]},
        {"id": "junk", "kind": "phase_line", "points": "nope"},
        {"id": "ok", "kind": "phase_line", "points": [[0.0, 0.0], [1.0, 1.0], ["x", 2.0]]}
    ])));
    assert_eq!(gs.len(), 1, "{gs:?}");
    assert_eq!(gs[0].id, "ok");
    assert_eq!(
        gs[0].points.len(),
        2,
        "the bad vertex is dropped, not zeroed"
    );

    assert!(tactical_graphics_from_env(&json!({"weather": "clear"})).is_empty());
    assert!(tactical_graphics_from_env(&json!(null)).is_empty());
    assert!(tactical_graphics_from_env(&env(json!("not an array"))).is_empty());
}

/// The draft's arithmetic — what gates the commit, and the reason it lives in this file at
/// all (see the "DRAW state machine's pure half" note above).
#[test]
fn a_draft_knows_how_many_vertices_it_still_needs() {
    let mut d = TacticalDraft {
        kind: "phase_line".to_string(),
        verts: Vec::new(),
    };
    assert_eq!(d.needed(), 2);
    d.verts.push((0.0, 0.0));
    assert_eq!(d.needed(), 1);
    d.verts.push((1.0, 1.0));
    assert_eq!(d.needed(), 0);
    // Past the floor stays at zero — `saturating_sub`, so an extra vertex never wraps to a
    // huge "still needed" count and locks the commit out.
    d.verts.push((2.0, 2.0));
    assert_eq!(d.needed(), 0);

    let c = TacticalDraft {
        kind: "curved_arrow".to_string(),
        verts: vec![(0.0, 0.0), (1.0, 1.0)],
    };
    assert_eq!(c.needed(), 1, "two points cannot curve");
}

/// The canvas floor and the compile floor are ONE function, walked over the core's own
/// vocabulary so a kind added there without one here fails by name.
#[test]
fn the_canvas_floor_is_the_core_validator_floor() {
    use website_map_engine::data::scenario::tactical_graphics::min_points;
    use website_map_engine::data::scenario::tactical_graphics::KINDS;
    assert_eq!(KINDS.len(), 4);
    for kind in KINDS {
        let floor = min_points(kind).unwrap_or_else(|| panic!("{kind} has no floor"));
        let draft = TacticalDraft {
            kind: (*kind).to_string(),
            verts: vec![(0.0, 0.0); floor],
        };
        assert_eq!(draft.needed(), 0, "{kind} must commit at its own floor");
        let short = TacticalDraft {
            kind: (*kind).to_string(),
            verts: vec![(0.0, 0.0); floor - 1],
        };
        assert_eq!(short.needed(), 1, "{kind} must refuse one under it");
    }
}

#[test]
fn a_minted_id_never_collides_with_a_live_row() {
    let rows = vec![
        json!({"id": "tg_pl_1"}),
        json!({"id": "tg_pl_2"}),
        json!({"id": "tg_ca_1"}),
    ];
    assert_eq!(mint_graphic_id(&rows, "phase_line"), "tg_pl_3");
    assert_eq!(mint_graphic_id(&rows, "curved_arrow"), "tg_ca_2");
    assert_eq!(mint_graphic_id(&rows, "axis_of_advance"), "tg_aoa_1");
    assert_eq!(mint_graphic_id(&[], "boundary"), "tg_b_1");
}

#[test]
fn a_bad_hex_colour_falls_back_to_the_kind_default() {
    assert_eq!(hex_to_rgb("#ff8000"), Some([1.0, 128.0 / 255.0, 0.0]));
    assert_eq!(hex_to_rgb("ff8000"), None);
    assert_eq!(hex_to_rgb("#ff800"), None);
    assert_eq!(hex_to_rgb("#gggggg"), None);
    let gs = tactical_graphics_from_env(&env(json!([
        {"id": "a", "kind": "boundary", "points": [[0.0, 0.0], [1.0, 0.0]],
         "style": {"color": "nope", "alpha": 4.0}}
    ])));
    assert_eq!(
        gs[0].rgba,
        default_kind_rgba("boundary"),
        "an unusable style must not draw black at alpha 4"
    );
}
