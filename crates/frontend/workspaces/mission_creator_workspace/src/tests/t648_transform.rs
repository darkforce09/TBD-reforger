use mission_creator_state::transform::{
    Axis, RING_HIT_TOL_PX, SnapState, TRANSLATE_LADDER_M, WIDGET_RADIUS_PX, WidgetVariant,
    press_on_ring, snap_translate, step,
};
use mission_operations::rotation::snap_value;
use mission_operations::rotation::{bearing_to_face, norm_deg, snap_rotate};

// ── QUANTISER: off-state passthrough ──────────────────────────────────────────────────────
#[test]
fn off_rung_is_passthrough() {
    // Rung 0 (OFF) returns the value byte-for-byte — a free move / free rotate.
    assert_eq!(snap_translate(3.7, 0), 3.7);
    assert_eq!(snap_translate(-123.456, 0), -123.456);
    // snap_value with a non-positive / non-finite step is also passthrough (the OFF branch).
    assert_eq!(snap_value(3.7, 0.0), 3.7);
    assert_eq!(snap_value(3.7, -5.0), 3.7);
    assert_eq!(snap_value(3.7, f64::NAN), 3.7);
    // Rotation OFF still NORMALISES to [0,360) (the stored range) but does not quantise.
    assert_eq!(snap_rotate(370.0, 0), 10.0);
    assert_eq!(snap_rotate(-30.0, 0), 330.0);
}

// ── QUANTISER: quantisation to a rung ─────────────────────────────────────────────────────
#[test]
fn snap_translate_quantises_to_the_rung() {
    // 5 m rung: 12 → 10, 13 → 15 (round to nearest multiple of 5).
    assert_eq!(snap_translate(12.0, 2), 10.0);
    assert_eq!(snap_translate(13.0, 2), 15.0);
    // 1 m rung: rounds to whole metres.
    assert_eq!(snap_translate(2.4, 1), 2.0);
    assert_eq!(snap_translate(2.6, 1), 3.0);
    // 10 m rung: negatives round symmetrically.
    assert_eq!(snap_translate(-14.0, 3), -10.0);
    assert_eq!(snap_translate(-16.0, 3), -20.0);
}

#[test]
fn snap_rotate_quantises_and_normalises() {
    // 45° rung: 40 → 45, 20 → 45? no — 20 rounds to 0 (nearest of {0,45}). 30 → 45.
    assert_eq!(snap_rotate(40.0, 3), 45.0);
    assert_eq!(snap_rotate(20.0, 3), 0.0);
    assert_eq!(snap_rotate(30.0, 3), 45.0);
    // 15° rung: 7 → 0, 8 → 15, 359 → 0 (360 normalises to 0).
    assert_eq!(snap_rotate(7.0, 2), 0.0);
    assert_eq!(snap_rotate(8.0, 2), 15.0);
    assert_eq!(snap_rotate(359.0, 2), 0.0);
    // 5° rung with wrap: 358 → 360 → 0.
    assert_eq!(snap_rotate(358.0, 1), 0.0);
}

// ── QUANTISER: increase/decrease clamping ─────────────────────────────────────────────────
#[test]
fn step_clamps_at_both_ends() {
    let len = TRANSLATE_LADDER_M.len(); // 4
    // Increase walks up and STOPS at the last rung.
    assert_eq!(step(0, len, 1), 1);
    assert_eq!(step(1, len, 1), 2);
    assert_eq!(step(2, len, 1), 3);
    assert_eq!(
        step(3, len, 1),
        3,
        "increase at the coarsest rung is a clamp, not a wrap"
    );
    // Decrease walks down and STOPS at OFF (0).
    assert_eq!(step(3, len, -1), 2);
    assert_eq!(step(1, len, -1), 0);
    assert_eq!(
        step(0, len, -1),
        0,
        "decrease at OFF is a clamp, not a wrap to the top"
    );
    // A zero delta is inert (still clamped into range).
    assert_eq!(step(2, len, 0), 2);
    // Degenerate empty ladder never panics.
    assert_eq!(step(0, 0, 1), 0);
}

// ── SnapState: the master latch + per-axis rungs ──────────────────────────────────────────
#[test]
fn snap_state_default_is_off_and_passthrough() {
    let s = SnapState::default();
    assert!(!s.enabled, "grid defaults OFF");
    assert_eq!(s.translate_rung, 0);
    assert_eq!(s.rotate_rung, 0);
    // Effective rungs are 0 while disabled REGARDLESS of the stored rung.
    let tuned = SnapState {
        enabled: false,
        translate_rung: 3,
        rotate_rung: 2,
    };
    assert_eq!(
        tuned.effective_translate_rung(),
        0,
        "grid off ⇒ translation passthrough even with a tuned rung"
    );
    assert_eq!(
        tuned.effective_rotate_rung(),
        0,
        "grid off ⇒ rotation passthrough"
    );
}

#[test]
fn toggling_the_latch_preserves_rungs() {
    let s = SnapState {
        enabled: false,
        translate_rung: 2,
        rotate_rung: 3,
    };
    let on = s.toggled();
    assert!(on.enabled);
    assert_eq!(
        on.translate_rung, 2,
        "toggle keeps the tuned translation rung"
    );
    assert_eq!(on.rotate_rung, 3, "toggle keeps the tuned rotation rung");
    assert_eq!(
        on.effective_translate_rung(),
        2,
        "enabled ⇒ tuned rung is live"
    );
    assert_eq!(on.effective_rotate_rung(), 3);
    assert!(!on.toggled().enabled, "toggling again turns it back off");
}

#[test]
fn stepping_a_rung_does_not_flip_the_latch() {
    // Stepping while OFF parks the rung without enabling (Eden keeps the two controls orthogonal).
    let s = SnapState::default().stepped(Axis::Translate, 1);
    assert!(!s.enabled, "stepping a rung must not enable the grid");
    assert_eq!(s.translate_rung, 1);
    assert_eq!(
        s.rotate_rung, 0,
        "stepping translation leaves rotation alone"
    );
    // Rotation axis is independent.
    let s2 = s.stepped(Axis::Rotate, 1).stepped(Axis::Rotate, 1);
    assert_eq!(s2.translate_rung, 1);
    assert_eq!(s2.rotate_rung, 2);
    // Clamps ride through SnapState too.
    let maxed = SnapState::default()
        .stepped(Axis::Rotate, 1)
        .stepped(Axis::Rotate, 1)
        .stepped(Axis::Rotate, 1)
        .stepped(Axis::Rotate, 1);
    assert_eq!(
        maxed.rotate_rung, 3,
        "clamped at the coarsest rotation rung"
    );
}

// ── SHIFT-ROTATE: face-cursor bearing golden (incl. wrap) ─────────────────────────────────
/// The bearing is yaw clockwise from north (+Y) — the doc/export convention. Cardinal goldens
/// plus the wrap case (west → 270, not −90).
#[test]
fn bearing_faces_the_cursor_clockwise_from_north() {
    let eps = 1e-9;
    // Pivot at origin; cursor at each cardinal.
    assert!(
        (bearing_to_face(0.0, 0.0, 0.0, 10.0).unwrap() - 0.0).abs() < eps,
        "north → 0°"
    );
    assert!(
        (bearing_to_face(0.0, 0.0, 10.0, 0.0).unwrap() - 90.0).abs() < eps,
        "east → 90°"
    );
    assert!(
        (bearing_to_face(0.0, 0.0, 0.0, -10.0).unwrap() - 180.0).abs() < eps,
        "south → 180°"
    );
    // West is the WRAP case: atan2 gives −90, normalise to 270.
    assert!(
        (bearing_to_face(0.0, 0.0, -10.0, 0.0).unwrap() - 270.0).abs() < eps,
        "west → 270° (the wrap: −90 must normalise, not stay negative)"
    );
    // A diagonal: NE → 45.
    assert!(
        (bearing_to_face(0.0, 0.0, 5.0, 5.0).unwrap() - 45.0).abs() < eps,
        "NE → 45°"
    );
    // Pivot offset from origin — bearing is relative to the pivot, not the world origin.
    assert!(
        (bearing_to_face(100.0, 200.0, 100.0, 250.0).unwrap() - 0.0).abs() < eps,
        "cursor due north of an offset pivot is still 0°"
    );
}

#[test]
fn bearing_is_none_for_a_degenerate_aim() {
    // Cursor exactly on the pivot → no meaningful bearing (the commit leaves rotation untouched).
    assert_eq!(bearing_to_face(50.0, 50.0, 50.0, 50.0), None);
    // Non-finite inputs → None, not a NaN commit.
    assert_eq!(bearing_to_face(0.0, 0.0, f64::NAN, 0.0), None);
    assert_eq!(bearing_to_face(0.0, 0.0, 0.0, f64::INFINITY), None);
}

#[test]
fn norm_deg_ranges_and_handles_nonfinite() {
    assert_eq!(norm_deg(0.0), 0.0);
    assert_eq!(norm_deg(360.0), 0.0);
    assert_eq!(norm_deg(370.0), 10.0);
    assert_eq!(norm_deg(-10.0), 350.0);
    assert_eq!(norm_deg(-370.0), 350.0);
    assert_eq!(norm_deg(f64::NAN), 0.0);
}

// ── WIDGET STATE MACHINE: 1/2/3 select, variant-gated gestures (T-795 Eden numbering) ────────
#[test]
fn widget_variant_matches_eden_numbering() {
    // T-795 — 1/2/3 map to Eden's widget row EXACTLY: No Widget / Translate / Rotate. Before this
    // ticket they were off by one (1=Translate, 2=Rotate, 3=nothing).
    let v = WidgetVariant::default();
    assert_eq!(v, WidgetVariant::Translate, "default variant is Translate");
    assert_eq!(v.select_digit(1), WidgetVariant::None, "1 → No Widget");
    assert_eq!(v.select_digit(2), WidgetVariant::Translate, "2 → Translate");
    assert_eq!(v.select_digit(3), WidgetVariant::Rotate, "3 → Rotate");
    // select_digit is total over the three real modes regardless of the current mode.
    assert_eq!(WidgetVariant::Rotate.select_digit(1), WidgetVariant::None);
    assert_eq!(WidgetVariant::None.select_digit(3), WidgetVariant::Rotate);
    // to_digit is the inverse — it selects the same digit that arms the variant (drives the
    // toolbar's three-way plate + the cursor hint).
    for m in [
        WidgetVariant::None,
        WidgetVariant::Translate,
        WidgetVariant::Rotate,
    ] {
        assert_eq!(
            WidgetVariant::default().select_digit(m.to_digit()),
            m,
            "from_digit(to_digit()) is the identity for every real mode"
        );
    }
    // 4/5 (Area Scaling / Area) and any other digit are INERT — reserved-unbound, no area-scale
    // variant yet (a transform selection is slots + vehicles, neither of which scales).
    assert_eq!(
        WidgetVariant::Rotate.select_digit(4),
        WidgetVariant::Rotate,
        "4 is reserved-unbound"
    );
    assert_eq!(
        WidgetVariant::Translate.select_digit(5),
        WidgetVariant::Translate,
        "5 is reserved-unbound"
    );
    assert_eq!(WidgetVariant::Rotate.select_digit(0), WidgetVariant::Rotate);
    // The cursor-adjacent mode hint labels each mode; they must read as the operator expects
    // (and match the toolbar / help wording). This also pins `label`, whose only other caller is
    // the wasm-only hint component.
    assert_eq!(WidgetVariant::None.label(), "No Widget");
    assert_eq!(WidgetVariant::Translate.label(), "Translate");
    assert_eq!(WidgetVariant::Rotate.label(), "Rotate");
}

#[test]
fn widget_variant_gates_its_gesture_axis() {
    // Only Rotate has a ring (a drag on the ring rotates; Shift+drag snaps to the rotation
    // ladder). None and Translate both move, so neither is a rotate.
    assert!(WidgetVariant::Rotate.is_rotate());
    assert!(!WidgetVariant::Translate.is_rotate());
    assert!(!WidgetVariant::None.is_rotate());
    // The step keys tune the axis matching the variant; None steps the translation ladder (a bare
    // drag still translates), Rotate the rotation ladder.
    assert_eq!(WidgetVariant::None.snap_axis(), Axis::Translate);
    assert_eq!(WidgetVariant::Translate.snap_axis(), Axis::Translate);
    assert_eq!(WidgetVariant::Rotate.snap_axis(), Axis::Rotate);
}

// ── T-795 — the rotate-ring hit-test geometry (the fix for the "ring is decoration" defect) ──
#[test]
fn press_on_ring_grabs_the_ring_band_only() {
    let (cx, cy) = (200.0, 150.0);
    // Dead on the ring stroke (due east of the pivot) → hit.
    assert!(
        press_on_ring(cx + WIDGET_RADIUS_PX, cy, cx, cy),
        "on the ring"
    );
    // Just inside / just outside the stroke, within tolerance → still a hit (the stroke is 2px;
    // a pixel-exact test would be un-hittable — that was half the decoration bug).
    assert!(press_on_ring(
        cx + WIDGET_RADIUS_PX - RING_HIT_TOL_PX + 0.5,
        cy,
        cx,
        cy
    ));
    assert!(press_on_ring(
        cx + WIDGET_RADIUS_PX + RING_HIT_TOL_PX - 0.5,
        cy,
        cx,
        cy
    ));
    // The CENTRE is not the ring — a press on the pivot dot must NOT rotate (it would be a
    // degenerate aim anyway); it falls through to the pick/move path.
    assert!(!press_on_ring(cx, cy, cx, cy), "the centre is not the ring");
    // Well outside the ring (empty ground beyond it) → miss, so a marquee can still start there.
    assert!(
        !press_on_ring(cx + WIDGET_RADIUS_PX + RING_HIT_TOL_PX + 20.0, cy, cx, cy),
        "beyond the band is empty ground — marquee territory"
    );
}
