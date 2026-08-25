use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
use crate::editor::mission_editor::transform::{
    bearing_to_face, norm_deg, press_on_ring, snap_rotate, snap_translate, snap_value, step, Axis,
    SnapState, WidgetVariant, RING_HIT_TOL_PX, ROTATE_LADDER_DEG, TRANSLATE_LADDER_M,
    WIDGET_RADIUS_PX,
};

/// Page-from-anchor + the T-934.13 gesture file (`canvas/gestures.rs`) — the transform wiring
/// spans the page body (keydown arms, widget mounts) and the moved pointer closures (the ring
/// promotion, the Shift-rotate arm, the Move commit). Each half scrubbed separately.
fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let mut src = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    src.push_str(&live_code(include_str!("../canvas/gestures.rs")));
    src.push_str(&live_code(include_str!("../canvas/commands.rs")));
    src
}

// ── QUANTISER: ladders ────────────────────────────────────────────────────────────────────
/// The ladders are exactly the ticket's rungs, OFF-first.
#[test]
fn ladders_are_the_ticket_rungs() {
    assert_eq!(
        TRANSLATE_LADDER_M,
        [0.0, 1.0, 5.0, 10.0],
        "translation ladder = off/1/5/10 m"
    );
    assert_eq!(
        ROTATE_LADDER_DEG,
        [0.0, 5.0, 15.0, 45.0],
        "rotation ladder = off/5/15/45°"
    );
    assert_eq!(Axis::Translate.ladder(), &TRANSLATE_LADDER_M);
    assert_eq!(Axis::Rotate.ladder(), &ROTATE_LADDER_DEG);
}

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

#[test]
fn status_readout_names_the_active_steps() {
    // O-10 — the chip reads SNAP (it names the snap grid, not the map grid).
    assert_eq!(SnapState::default().status_readout(), "SNAP  off");
    let s = SnapState {
        enabled: true,
        translate_rung: 2, // 5 m
        rotate_rung: 2,    // 15°
    };
    assert_eq!(s.status_readout(), "SNAP  move 5 m \u{b7} rot 15\u{b0}");
    let off = SnapState {
        enabled: true,
        translate_rung: 0,
        rotate_rung: 0,
    };
    assert_eq!(
        off.status_readout(),
        "SNAP  move off \u{b7} rot off",
        "an enabled grid with both ladders at OFF reads 'off' per axis"
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
    assert_eq!(v.from_digit(1), WidgetVariant::None, "1 → No Widget");
    assert_eq!(v.from_digit(2), WidgetVariant::Translate, "2 → Translate");
    assert_eq!(v.from_digit(3), WidgetVariant::Rotate, "3 → Rotate");
    // from_digit is total over the three real modes regardless of the current mode.
    assert_eq!(WidgetVariant::Rotate.from_digit(1), WidgetVariant::None);
    assert_eq!(WidgetVariant::None.from_digit(3), WidgetVariant::Rotate);
    // to_digit is the inverse — it selects the same digit that arms the variant (drives the
    // toolbar's three-way plate + the cursor hint).
    for m in [
        WidgetVariant::None,
        WidgetVariant::Translate,
        WidgetVariant::Rotate,
    ] {
        assert_eq!(
            WidgetVariant::default().from_digit(m.to_digit()),
            m,
            "from_digit(to_digit()) is the identity for every real mode"
        );
    }
    // 4/5 (Area Scaling / Area) and any other digit are INERT — reserved-unbound, no area-scale
    // variant yet (a transform selection is slots + vehicles, neither of which scales).
    assert_eq!(
        WidgetVariant::Rotate.from_digit(4),
        WidgetVariant::Rotate,
        "4 is reserved-unbound"
    );
    assert_eq!(
        WidgetVariant::Translate.from_digit(5),
        WidgetVariant::Translate,
        "5 is reserved-unbound"
    );
    assert_eq!(WidgetVariant::Rotate.from_digit(0), WidgetVariant::Rotate);
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

// ── KEYDOWN CENSUS: G free (+ brackets + digits), Space stays flyTo ────────────────────────
/// The two window-level EDITOR keydowns are this file's and `mission_history`'s. Census both as
/// raw text (keeping string literals — a keydown arm IS a `"KeyX"` string). T-648's new keys must
/// be free before this slice, and Space must remain `center_on_selection` (flyTo), not a widget
/// cycle (the collision decision).
#[test]
fn t648_keydown_census() {
    // Slice ONLY the editor keydown MATCH of each of the two window-level editor keydowns, so a
    // needle can never self-match inside this test module (which sits in the same file). The arm
    // list runs from the `match ev.code().as_str()` head to the arm-list terminator `_ => false`
    // / `_ => {}`. Comments are stripped (`live_source` keeps the `"KeyX"` arm LITERALS but drops
    // notes) so a comment that MENTIONS a rejected keysym for explanation is not read as a
    // binding — the census is about arm patterns, not prose.
    //
    // T-703/T-738: the slicer used to be a private copy right here — one of FOUR. It now lives
    // once, in `eden_help::keymap_census`, beside the structured (code, modifiers) census that
    // detects collisions; `there_is_exactly_one_extractor` keeps it from being copied again.
    use crate::editor::panels::help_modal::keymap_census::keydown_arms;
    let this_arms = keydown_arms(include_str!("../canvas/commands.rs"));
    let history_arms = keydown_arms(include_str!("../state/history.rs"));
    // Needles assembled so the LITERAL never appears verbatim in this test's own source.
    let key = |k: &str| format!("\"{k}\"");
    let g = key("KeyG");
    let bl = key("BracketLeft");
    let br = key("BracketRight");
    let d1 = key("Digit1");
    let d2 = key("Digit2");
    let d3 = key("Digit3");
    let d4 = key("Digit4");
    let d5 = key("Digit5");
    let semicolon = key("Semicolon");
    let odiaeresis = format!("odi{}", "aeresis"); // split so it is not a verbatim literal here

    // The other editor keydown (Ctrl+Z/Y) must NOT claim any T-648/T-795 key.
    assert!(
        !history_arms.contains(&g)
            && !history_arms.contains(&bl)
            && !history_arms.contains(&br)
            && !history_arms.contains(&d1)
            && !history_arms.contains(&d2)
            && !history_arms.contains(&d3),
        "census: mission_history's keydown (Ctrl+Z/Y) must not claim G / [ / ] / 1 / 2 / 3"
    );
    // G is the chosen grid toggle — an arm here, and NOT an Eden keysym artefact.
    assert!(
        this_arms.contains(&format!("{g} if !modk")),
        "KEY-GRID-001: G must be the grid-toggle keydown arm"
    );
    assert!(
        !this_arms.contains(&odiaeresis) && !this_arms.contains(&semicolon),
        "census: must NOT copy Eden's odiaeresis / ; keysym artefacts for the grid toggle"
    );
    // [ / ] step the snap rung.
    assert!(
        this_arms.contains(&format!("{bl} if !modk"))
            && this_arms.contains(&format!("{br} if !modk")),
        "TOOLBAR-GRID-MOVE-001: [ and ] must be the decrease/increase keydown arms"
    );
    // 1 / 2 / 3 select the widget variant, numbered to match Eden's row (No Widget / Translate /
    // Rotate — T-795). Eden's free direct keys — the Space collision decision.
    assert!(
        this_arms.contains(&format!("{d1} if !modk"))
            && this_arms.contains(&format!("{d2} if !modk"))
            && this_arms.contains(&format!("{d3} if !modk")),
        "WIDGET-CYCLE-001: 1, 2 and 3 must be the widget-variant keydown arms (T-795 Eden numbering)"
    );
    // 4 / 5 (Area Scaling / Area) stay RESERVED-UNBOUND — no keydown arm binds them; a future
    // area-scale slice adds them without renumbering. A `Digit4`/`Digit5` arm appearing here would
    // mean the reservation was quietly spent.
    assert!(
        !this_arms.contains(&format!("{d4} if !modk"))
            && !this_arms.contains(&format!("{d5} if !modk")),
        "T-795: Digit4/Digit5 are reserved-unbound (Eden's Area Scaling / Area) — no arm yet"
    );
    // Space STAYS flyTo — it must still map to center_on_selection and must NOT cycle the widget.
    let space = key("Space");
    assert!(
        this_arms.contains(&format!(
            "{space} if !modk => editor_ops::center_on_selection()"
        )),
        "collision decision: Space must remain flyTo (center_on_selection), not a widget cycle"
    );
    // The Space arm must not touch widget_variant (it is a one-liner flyTo call).
    let space_at = this_arms.find(&space).expect("Space arm present");
    let space_arm = &this_arms[space_at..(space_at + 120).min(this_arms.len())];
    assert!(
        !space_arm.contains("widget_variant"),
        "collision decision: the Space arm must not cycle the widget variant"
    );
}

// ── SOURCE PINS: the Shift-rotate gesture arm ─────────────────────────────────────────────
/// Shift+drag on a SELECTED entity promotes to `LG::Rotate` (not `LG::Move`), and the commit
/// routes through `rotate_selection_to_face` — never the atomic translate `move_entities_*`.
#[test]
fn shift_rotate_arm_promotes_and_commits_through_the_field_write() {
    let ed = editor_live();
    assert!(
        ed.contains("ev.shift_key()") && ed.contains("LG::Rotate {"),
        "XFORM-SHIFT-001: a Shift-held drag on a selected entity must open LG::Rotate"
    );
    assert!(
        ed.contains("editor_ops::rotate_selection_to_face("),
        "the LG::Rotate commit must call rotate_selection_to_face"
    );
    // Isolate the pointerup LG::Rotate arm and prove it commits rotation, NOT a translate.
    let rot_arm = {
        let at = ed
            .find("LG::Rotate { cam, .. } =>")
            .expect("the pointerup LG::Rotate commit arm is present");
        let rest = &ed[at..];
        let end = rest[3..].find("LG::").map(|i| i + 3).unwrap_or(rest.len());
        &rest[..end]
    };
    assert!(
        rot_arm.contains("rotate_selection_to_face("),
        "the rotate commit arm must call the field-write rotate"
    );
    assert!(
        !rot_arm.contains("move_entities_and_vehicles(") && !rot_arm.contains("move_entities("),
        "the rotate arm must NOT translate — rotation rides the attrs/vehicle field write"
    );
}

/// The atomic move-commit pin's invariant is UNDISTURBED by the new arm: exactly one `LG::Move`
/// arm still calls `move_entities_and_vehicles`, and `LG::Rotate` is a separate arm. (The
/// authoritative version of this pin lives in map-engine-core/doc/store.rs and runs under
/// `cargo test -p map-engine-core`; this is the frontend-local echo so a fork shows up here too.)
#[test]
fn only_one_move_arm_commits_the_atomic_mix() {
    let ed = editor_live();
    let move_arms: Vec<&str> = ed
        .split("LG::Move")
        .skip(1)
        .map(|s| s.split("LG::").next().unwrap_or(s))
        .filter(|arm| arm.contains(".move_entities_and_vehicles("))
        .collect();
    assert_eq!(
        move_arms.len(),
        1,
        "exactly one LG::Move arm may commit via move_entities_and_vehicles (found {})",
        move_arms.len()
    );
}

/// **wave-127 F-6** — the drag commit must carry each dragged slot's CURRENT z.
///
/// `move_entities_in_txn` (map-engine-core) reads the existing z, DISCARDS it, and writes the
/// caller's `zs[i]` verbatim — so a `vec![0.0; n]` here is not a placeholder, it is a write of
/// `0.0` onto every dragged slot inside one txn, with nothing left in this frontend to
/// re-sample terrain afterwards (`terrainZ` did not survive the React deletion). Vehicles in
/// the same drag keep their z, which is the asymmetry that gives the defect away.
///
/// This reads the LIVE `LG::Move` commit arm — `live_code` strips comments and dead code and
/// cuts the test module, so neither a reassuring note nor this module's own text can satisfy
/// it. It requires the zeros gone, the SHARED `keep_z_rows`/`slot_z` pair used (a third
/// z-resolution path is its own defect class here), and `zs` built by mapping over the same
/// `slot_ids` that is then passed as `ids` — the structural fact that makes `zs[i]` the z of
/// `slot_ids[i]`. A mismatched zip would hand one slot another slot's elevation, which is a
/// worse outcome than the zeroing this fixes.
#[test]
fn drag_move_commit_carries_each_slots_current_z() {
    let ed = editor_live();
    let arm = ed
        .split("LG::Move")
        .skip(1)
        .map(|s| s.split("LG::").next().unwrap_or(s))
        .find(|arm| arm.contains(".move_entities_and_vehicles("))
        .expect("the LG::Move commit arm is present");
    let flat: String = arm.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        !flat.contains("vec![0.0;"),
        "wave-127 F-6: the drag must not pass a zero-filled `zs` — the core writes it verbatim, \
         so that is a flatten of every dragged slot's authored z, not a placeholder"
    );
    assert!(
        flat.contains("keep_z_rows(") && flat.contains("slot_z("),
        "the drag must resolve z through the shared keep_z_rows/slot_z pair (exact f64 off the \
         raw row, hidden-layer slots included), not a third z-resolution path"
    );
    assert!(
        flat.contains("slot_ids.iter().map("),
        "`zs` must be built by mapping over `slot_ids` ITSELF, in order — that is what pins \
         zs[i] to slot_ids[i]"
    );
    assert!(
        flat.contains("move_entities_and_vehicles(slot_ids,&veh_ids,dx,dy,zs"),
        "the resolved `zs` must be the vector handed to the translate, positionally after dx/dy"
    );

    // The two statements must stay ADJACENT. The asserts above prove `zs` is mapped from
    // `slot_ids` and that `slot_ids` is what gets translated — but neither can see an edit
    // BETWEEN them. An inserted `slot_ids.sort()` would repoint every z by one slot, handing
    // each entity a neighbour's elevation, and every assert above would still pass. That is a
    // worse outcome than the zeroing F-6 fixed, so the window itself is pinned.
    let between = flat
        .split("slot_ids.iter().map(")
        .nth(1)
        .and_then(|s| s.split("move_entities_and_vehicles(").next())
        .expect("the `zs` build must precede the translate");
    for mutator in [
        "slot_ids.sort",
        "slot_ids.reverse",
        "slot_ids.dedup",
        "slot_ids.retain",
        "slot_ids.swap",
        "slot_ids.remove",
        "slot_ids.truncate",
        "slot_ids.push",
        "slot_ids.insert",
        "slot_ids.clear",
        "slot_ids.drain",
    ] {
        assert!(
            !between.contains(mutator),
            "wave-127 NIT: `{mutator}` between the `zs` build and the translate would reorder \
             or resize `slot_ids` after `zs` was built, silently giving each slot another \
             slot's z while every other assertion in this test stayed green"
        );
    }
}

// ── SOURCE PINS: the keydown bindings drive the right state ────────────────────────────────
#[test]
fn keydown_arms_drive_snap_and_variant_state() {
    let ed = editor_live();
    assert!(
        ed.contains("snap.set(snap.get_untracked().toggled())"),
        "G must toggle the SnapState master latch"
    );
    assert!(
        ed.contains("snap.set(snap.get_untracked().stepped(axis, -1))")
            && ed.contains("snap.set(snap.get_untracked().stepped(axis, 1))"),
        "[ / ] must step the current-variant snap axis down / up"
    );
    assert!(
        ed.contains("widget_variant.set(widget_variant.get_untracked().from_digit(1))")
            && ed.contains("widget_variant.set(widget_variant.get_untracked().from_digit(2))")
            && ed.contains("widget_variant.set(widget_variant.get_untracked().from_digit(3))"),
        "1 / 2 / 3 must set the widget variant (T-795 Eden numbering: No Widget / Translate / Rotate)"
    );
}

// ── SOURCE PINS: the widget + snap-readout mounts ─────────────────────────────────────────
#[test]
fn widget_and_readout_are_mounted() {
    let ed = editor_live();
    assert!(
        ed.contains("TransformWidgetOverlay") && ed.contains("register_widget_pivot("),
        "WIDGET-TRANS-001: the transform widget must be mounted and its pivot registered"
    );
    assert!(
        ed.contains("SnapReadout"),
        "TOOLBAR-GRID-MOVE-001: the snap-step readout must be mounted"
    );
    // The Shift-rotate commit rung comes from the EFFECTIVE (grid-gated) rotation rung.
    assert!(
        ed.contains("effective_rotate_rung()"),
        "the rotate commit must quantise to the grid-gated rotation rung"
    );
}

// ── SOURCE PIN: the included one-line comment fix (before/after) ───────────────────────────
/// The wave-109 verifier's binding fix: the false T-159.22 claim must be GONE and replaced by the
/// truth (has_pending short-circuits regardless of left/pan_px). Pinned on the RAW file — the
/// claim and its correction are comments, which `live_code` strips.
#[test]
fn false_t159_22_comment_is_corrected() {
    // T-934.13 moved the pointerup closure (whose comment this pins) to canvas/gestures.rs; the
    // negative check keeps sweeping BOTH files so the false claim cannot re-enter either.
    let raw = concat!(
        include_str!("../mission_editor.rs"),
        include_str!("../canvas/gestures.rs")
    );
    // The false-claim needle is assembled from fragments so this test's OWN source (in this same
    // file, read via include_str!) is not a decoy match for it.
    let false_claim = format!(
        "{}{}",
        "`left`/`pan_px` are both None here", " and no gesture branch below would fire"
    );
    assert!(
        !raw.contains(&false_claim),
        "the false 'both-None here' T-159.22 claim must be deleted or corrected"
    );
    // The correction needle is fragment-assembled for the same hygiene. (Pre-T-934.9, when this
    // test lived inside mission_editor.rs, an earlier phrasing of this needle was satisfied by
    // its OWN string literal — the evacuation exposed that decoy; this pins the REAL corrected
    // comment on the pointerup closure.)
    let correction = format!("{}{}", "The ARMED state (`has_pending()`)", " is checked");
    assert!(
        raw.contains(&correction) && raw.contains("before any gesture branch below"),
        "the correction must state the true invariant: the armed check runs before any gesture \
         branch"
    );
}

// ── FIRED RULE: the quantiser is load-bearing (perturb / fail / restore) ───────────────────
/// Fire the quantiser once: a build that quantises everyday (perturb `snap_value` to always
/// passthrough) must FAIL the quantisation goldens. This proves the ladders actually bite — a
/// green suite over a no-op quantiser would be worthless. Restore is implicit (in-memory reasoning
/// via a re-derived value); the real `snap_value` is exercised by the goldens above.
#[test]
fn fired_rule_quantiser_is_load_bearing() {
    // The real quantiser bites: 12 m at the 5 m rung lands on 10.
    assert_eq!(
        snap_translate(12.0, 2),
        10.0,
        "canary: the real quantiser snaps"
    );
    // Perturbation model: a passthrough quantiser (the regression) would return the input.
    let passthrough = |v: f64, _step: f64| v;
    let perturbed = passthrough(12.0, TRANSLATE_LADDER_M[2]);
    assert_ne!(
        perturbed, 10.0,
        "fired rule: a passthrough quantiser (snap off everywhere) does NOT land on the grid — \
         so the quantisation goldens above genuinely constrain the snap, they are not vacuous"
    );
    // And the rotation ladder likewise bites (40° → 45° at the 45° rung).
    assert_eq!(snap_rotate(40.0, 3), 45.0);
    assert_ne!(
        40.0, 45.0,
        "fired rule: the rotation snap moved the value — the golden is not an identity"
    );
}

// ── SOURCE PINS on the pure module living where it can be native-tested ────────────────────
/// The `transform` module is UNGATED (native-testable) — the whole reason these behavioural tests
/// run at all. Pin that placement so a refactor into wasm-only `select_tool` (where a native
/// `cargo test` would silently skip them) is caught.
#[test]
fn transform_module_is_native_testable() {
    let raw = include_str!("../mission_editor.rs");
    // The module declaration must NOT sit under a wasm cfg.
    let decl = "pub mod transform {";
    let at = raw.find(decl).expect("transform module present");
    let before = &raw[at.saturating_sub(60)..at];
    assert!(
        !before.contains("cfg(target_arch = \"wasm32\")"),
        "the transform module must stay ungated so its quantiser/bearing tests run on native \
         `cargo test -p website-frontend` (the command the wave gate uses)"
    );
    // And the rotate commit really rides the existing field write, per the ticket.
    let ops = include_str!("../state/operations/transform.rs");
    let ops_live = live_code(ops);
    let body = only_body(&ops_live, "pub fn rotate_selection_to_face(");
    assert!(
        body.contains("rotate_entities("),
        "rotate_selection_to_face must commit via MissionDocCore::rotate_entities (T-732 one-txn batch)"
    );
}
