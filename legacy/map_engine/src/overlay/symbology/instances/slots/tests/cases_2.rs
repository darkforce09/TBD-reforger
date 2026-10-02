//! Role: Domain regression cases.
//! Position: `overlay/symbology/instances/slots/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

#[test]
fn vehicle_symbology_carries_kind_heading_and_side() {
    let xy = [0.0_f32, 0.0, 10.0, 10.0, 20.0, 20.0];
    let aliases = vec![
        "M1025".to_string(),
        "M923A1".to_string(),
        "M113A3".to_string(),
    ];
    let tints = [SIDE_BLUFOR_RGBA, SIDE_OPFOR_RGBA, SIDE_INDFOR_RGBA];
    let b = pack_vehicle_symbology(&xy, &aliases, &tints, &[0.0, 90.0, 270.0], 1.0, 11);
    assert_eq!(b.len(), 3 * SLOT_ICON_STRIDE);
    let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];
    let glyph = |i: usize| u16::from_le_bytes(row(i)[14..16].try_into().unwrap());
    assert_eq!(
        glyph(0),
        11 + VEHICLE_CELL_BASE + VehicleKind::WheeledLight as u16
    );
    assert_eq!(glyph(1), 11 + VEHICLE_CELL_BASE + VehicleKind::Truck as u16);
    assert_eq!(glyph(2), 11 + VEHICLE_CELL_BASE + VehicleKind::Apc as u16);
    assert_ne!(glyph(0), glyph(1));
    assert_ne!(glyph(1), glyph(2));
    assert_eq!(
        i16::from_le_bytes(row(1)[12..14].try_into().unwrap()),
        yaw_to_snorm16(-90.0)
    );
    assert_eq!(
        u32::from_le_bytes(row(2)[16..20].try_into().unwrap()),
        pack_rgba_u32(SIDE_INDFOR_RGBA)
    );

    let far = pack_vehicle_symbology(&xy, &aliases, &tints, &[0.0, 90.0, 270.0], 99.0, 11);
    assert_eq!(
        u16::from_le_bytes(far[14..16].try_into().unwrap()),
        SLOT_GLYPH_DISC
    );
    assert!(pack_vehicle_symbology(&[], &[], &[], &[], 1.0, 11).is_empty());
}

#[test]
fn symbology_tolerates_short_parallel_columns() {
    let xy = [0.0_f32, 0.0, 1.0, 1.0, 2.0, 2.0];
    let b = pack_slot_symbology(&xy, &[true], &[SIDE_OPFOR_RGBA], &[], &[10.0], 1.0, 11);
    assert_eq!(b.len(), 3 * SLOT_ICON_STRIDE, "every row still packs");
    let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];

    assert_eq!(
        u16::from_le_bytes(row(0)[14..16].try_into().unwrap()),
        11 + UNIT_SELECTED_CELL_BASE + UnitRoleClass::Rifleman as u16
    );

    assert_eq!(
        u32::from_le_bytes(row(2)[16..20].try_into().unwrap()),
        pack_rgba_u32(SIDE_BLUFOR_RGBA)
    );
    assert_eq!(i16::from_le_bytes(row(2)[12..14].try_into().unwrap()), 0);
    assert!(pack_slot_symbology(&[], &[], &[], &[], &[], 1.0, 11).is_empty());
}

#[test]
fn symbology_ids_fit_the_shader_glyph_clamp() {
    const ATLAS_GLYPH_COUNT: u16 = 32;
    const MARKER_GLYPH_COUNT: u16 = 11;
    #[allow(clippy::cast_possible_truncation)]
    let last = MARKER_GLYPH_COUNT + SYMBOLOGY_CELL_COUNT as u16 - 1;
    assert_eq!(last, 25);
    assert!(
        last < ATLAS_GLYPH_COUNT,
        "symbology overflows the 32-cell UV table"
    );

    #[allow(clippy::cast_possible_truncation)]
    {
        assert_eq!(
            UNIT_SELECTED_CELL_BASE,
            UNIT_CELL_BASE + UNIT_ROLE_CLASS_COUNT as u16
        );
        assert_eq!(
            VEHICLE_CELL_BASE,
            UNIT_SELECTED_CELL_BASE + UNIT_ROLE_CLASS_COUNT as u16
        );
        assert_eq!(COMMENT_CELL, VEHICLE_CELL_BASE + VEHICLE_KIND_COUNT as u16);
        assert_eq!(COMMENT_SELECTED_CELL, COMMENT_CELL + 1);
        assert_eq!(SYMBOLOGY_CELL_COUNT as u16, COMMENT_SELECTED_CELL + 1);
    }
}

#[test]
fn slot_atlas_ring_and_disc_probes() {
    let a = build_slot_atlas();
    let alpha = |x: usize, y: usize| a.rgba[(y * 128 + x) * 4 + 3];

    assert_eq!(alpha(32, 32), 0, "ring center must be hollow");
    assert_eq!(alpha(32 + 17, 32), 255, "ring band must be opaque");
    assert_eq!(alpha(32 + 30, 32), 0, "outside ring must be transparent");

    assert_eq!(alpha(96, 32), 255, "disc center must be opaque");
    assert_eq!(alpha(96 + 20, 32), 255, "disc mid must be opaque");
    assert_eq!(alpha(96 + 30, 32), 0, "outside disc must be transparent");

    assert_eq!(&a.rgba[0..3], &[255, 255, 255]);
}
