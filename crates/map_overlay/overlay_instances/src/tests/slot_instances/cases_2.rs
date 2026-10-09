//! Slot instance cases: vehicle symbology, short columns, the shader glyph clamp and the slot
//! atlas probes.

use super::*;

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
    #[allow(clippy::cast_possible_truncation)]
    let last = unit_symbology::markers::MARKER_GLYPH_COUNT as u16 + SYMBOLOGY_CELL_COUNT as u16 - 1;
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
