//! Slot instance cases: ring and disc packing, side tints, unit and vehicle symbology, headings
//! and the symbol atlas extension.

use super::*;

#[test]
fn icon_stride_is_20() {
    assert_eq!(SLOT_ICON_STRIDE, 20);
    let one = pack_one_slot(1.5, -2.5, false);
    assert_eq!(one.len(), 20);
}

#[test]
fn pack_count_matches_xy() {
    let xy = [0.0_f32, 0.0, 100.0, 200.0, 300.0, 400.0];
    let sel = [false, true, false];
    let bytes = pack_slot_instances(&xy, &sel, &[]);
    assert_eq!(bytes.len(), 3 * SLOT_ICON_STRIDE);

    let size1 = f32::from_le_bytes(bytes[20 + 8..20 + 12].try_into().unwrap());
    assert!((size1 - SLOT_SELECTED_PX).abs() < 1e-6);
    let tint1 = u32::from_le_bytes(bytes[20 + 16..20 + 20].try_into().unwrap());
    assert_eq!(tint1, pack_rgba_u32(SLOT_SELECTED_RGBA));

    let size0 = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
    assert!((size0 - SLOT_RING_PX).abs() < 1e-6);
    assert_eq!(
        u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        pack_rgba_u32(SLOT_PRIMARY_RGBA)
    );
}

#[test]
fn selected_overrides_side_tint() {
    let xy = [10.0_f32, 20.0];
    let bytes = pack_rings(&xy, &[true], &["OPFOR"]);
    let tint = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    assert_eq!(tint, pack_rgba_u32(SLOT_SELECTED_RGBA));
    let size = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
    assert!((size - SLOT_SELECTED_PX).abs() < 1e-6);
}

#[test]
fn drag_delta_math() {
    let (x, y) = drag_projected(100.0, 200.0, 3.5, -1.25);
    assert!((x - 103.5).abs() < 1e-12);
    assert!((y - 198.75).abs() < 1e-12);
}

#[test]
fn classify_drag_transition_truth_table() {
    assert_eq!(
        classify_drag_transition(false, true, true, false),
        DragGpuPhase::Start
    );
    assert_eq!(
        classify_drag_transition(true, true, false, true),
        DragGpuPhase::Delta
    );
    assert_eq!(
        classify_drag_transition(true, false, true, true),
        DragGpuPhase::End
    );
    assert_eq!(
        classify_drag_transition(true, true, true, false),
        DragGpuPhase::Restart
    );
    assert_eq!(
        classify_drag_transition(true, true, false, false),
        DragGpuPhase::Idle
    );
    assert_eq!(
        classify_drag_transition(false, false, false, true),
        DragGpuPhase::Idle
    );
}

#[test]
fn pack_selection_only_dense_k() {
    let xy = [0.0_f32, 0.0, 100.0, 200.0, 300.0, 400.0];
    let sel = [false, true, true];
    let bytes = pack_selection_only(&xy, &sel);
    assert_eq!(bytes.len(), 2 * SLOT_ICON_STRIDE);
    let size0 = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
    assert!((size0 - SLOT_SELECTED_PX).abs() < 1e-6);
    let x0 = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
    assert!((x0 - 100.0).abs() < 1e-6);
}

#[test]
fn selected_mask_from_set() {
    let ids = vec!["a".into(), "b".into(), "c".into()];
    let mut set = HashSet::new();
    set.insert("b".into());
    assert_eq!(selected_mask(&ids, &set), vec![false, true, false]);
}

#[test]
fn slot_lane_carries_yaw_and_the_sign_is_the_compass_convention() {
    let yaw_of = |bytes: &[u8]| i16::from_le_bytes(bytes[12..14].try_into().unwrap());
    let xy = [100.0_f32, 200.0];
    let roles = vec!["kit:us_rifleman".to_string()];
    let pack = |h: f32| pack_slot_symbology(&xy, &[false], &[], &roles, &[h], 1.0, 11);

    let h90 = pack(90.0);
    let h180 = pack(180.0);
    assert_ne!(h90, h180, "heading 90 and 180 must not pack identically");

    assert_eq!(yaw_of(&h90), yaw_to_snorm16(-90.0));
    assert!(yaw_of(&h90) < 0, "east must rotate the point clockwise");
    assert_eq!(yaw_of(&h180), yaw_to_snorm16(-180.0));
    assert_eq!(yaw_of(&pack(0.0)), 0, "north is the un-rotated cell");

    assert_eq!(
        yaw_of(&pack_slot_symbology(
            &xy,
            &[false],
            &[],
            &roles,
            &[],
            1.0,
            11
        )),
        0
    );

    assert_eq!(yaw_of(&pack_slot_instances(&xy, &[false], &[])), 0);
}

#[test]
fn selection_layers_over_the_role_glyph_and_keeps_heading() {
    let xy = [0.0_f32, 0.0, 50.0, 50.0];
    let roles = vec!["kit:us_medic".to_string(), "kit:us_medic".to_string()];
    let b = pack_slot_symbology(
        &xy,
        &[false, true],
        &[SIDE_OPFOR_RGBA, SIDE_OPFOR_RGBA],
        &roles,
        &[45.0, 45.0],
        1.0,
        11,
    );
    let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];
    let glyph = |i: usize| u16::from_le_bytes(row(i)[14..16].try_into().unwrap());
    let size = |i: usize| f32::from_le_bytes(row(i)[8..12].try_into().unwrap());
    let tint = |i: usize| u32::from_le_bytes(row(i)[16..20].try_into().unwrap());
    let yaw = |i: usize| i16::from_le_bytes(row(i)[12..14].try_into().unwrap());

    assert_eq!(glyph(0), 11 + UNIT_CELL_BASE + UnitRoleClass::Medic as u16);
    assert_eq!(
        glyph(1),
        11 + UNIT_SELECTED_CELL_BASE + UnitRoleClass::Medic as u16,
        "the selected cell must be the SAME role, ringed — not a generic ring"
    );
    assert!((size(0) - SLOT_UNIT_PX).abs() < 1e-6);
    assert!((size(1) - SLOT_SELECTED_PX).abs() < 1e-6);
    assert_eq!(tint(0), pack_rgba_u32(SIDE_OPFOR_RGBA), "side colour kept");
    assert_eq!(tint(1), pack_rgba_u32(SLOT_SELECTED_RGBA));
    assert_eq!(yaw(0), yaw(1), "selection must not drop the heading");
    assert_eq!(yaw(1), yaw_to_snorm16(-45.0));
}
