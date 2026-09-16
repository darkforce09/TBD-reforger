//! Role: Domain regression cases.
//! Position: `overlay/symbology/instances/slots/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

#[test]
fn icon_stride_is_20() {
    assert_eq!(SLOT_ICON_STRIDE, 20);
    let one = pack_one_slot(1.5, -2.5, false);
    assert_eq!(one.len(), 20);
}

#[test]
fn pack_vehicle_instances_disc_yellow() {
    let xy = [6400.0_f32, 6370.0];
    let bytes = pack_vehicle_instances(&xy);
    assert_eq!(bytes.len(), SLOT_ICON_STRIDE);
    let size = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
    assert!((size - SLOT_RING_PX).abs() < 1e-6);
    let glyph = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
    assert_eq!(glyph, SLOT_GLYPH_DISC);
    let tint = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    assert_eq!(tint, pack_rgba_u32(SLOT_SELECTED_RGBA));
    assert!(pack_vehicle_instances(&[]).is_empty());
}

#[test]
fn vehicle_first_paint_at_editor_default_zoom_is_silhouette_not_disc() {
    let m_per_px = px_to_m_at_zoom(-2.0);
    assert!(
        (m_per_px - 4.0).abs() < 1e-6,
        "INITIAL_ZOOM=-2 must be 4 m/px; got {m_per_px}"
    );
    assert!(
        symbology_visible(m_per_px),
        "default editor zoom must be inside SYMBOLOGY_MAX_M_PER_PX so first paint can be a glyph"
    );
    let xy = [6400.0_f32, 6370.0];
    let aliases = vec!["veh:us_m1025".to_string()];
    let tints = [SIDE_BLUFOR_RGBA];
    let bytes = pack_vehicle_symbology(&xy, &aliases, &tints, &[0.0], m_per_px, 11);
    assert_eq!(bytes.len(), SLOT_ICON_STRIDE);
    let glyph = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
    assert_ne!(
        glyph, SLOT_GLYPH_DISC,
        "T-930: first paint at default zoom must not be the yellow disc; glyph={glyph}"
    );
    assert_eq!(
        glyph,
        11 + VEHICLE_CELL_BASE + VehicleKind::WheeledLight as u16
    );
    let disc = pack_vehicle_instances(&xy);
    let disc_glyph = u16::from_le_bytes(disc[14..16].try_into().unwrap());
    assert_eq!(disc_glyph, SLOT_GLYPH_DISC);
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
fn side_tint_three_distinct() {
    assert_eq!(SIDE_BLUFOR_RGBA, [173, 198, 255, 255]);
    assert_eq!(SIDE_OPFOR_RGBA, [248, 113, 113, 255]);
    assert_eq!(SIDE_INDFOR_RGBA, [34, 197, 94, 255]);
    assert_eq!(SLOT_SELECTED_RGBA, [250, 204, 21, 255]);
    assert_eq!(SLOT_PRIMARY_RGBA, SIDE_BLUFOR_RGBA);
    assert_ne!(SIDE_BLUFOR_RGBA, SIDE_OPFOR_RGBA);
    assert_ne!(SIDE_BLUFOR_RGBA, SIDE_INDFOR_RGBA);
    assert_ne!(SIDE_OPFOR_RGBA, SIDE_INDFOR_RGBA);
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
fn pack_rings_side_tints() {
    let xy = [0.0_f32, 0.0, 1.0, 1.0, 2.0, 2.0];
    let bytes = pack_rings(&xy, &[false, false, false], &["BLUFOR", "OPFOR", "INDFOR"]);
    assert_eq!(bytes.len(), 3 * SLOT_ICON_STRIDE);
    let t0 = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let t1 = u32::from_le_bytes(bytes[20 + 16..20 + 20].try_into().unwrap());
    let t2 = u32::from_le_bytes(bytes[40 + 16..40 + 20].try_into().unwrap());
    assert_eq!(t0, pack_rgba_u32(SIDE_BLUFOR_RGBA));
    assert_eq!(t1, pack_rgba_u32(SIDE_OPFOR_RGBA));
    assert_eq!(t2, pack_rgba_u32(SIDE_INDFOR_RGBA));
    assert_ne!(t0, t1);
    assert_ne!(t0, t2);
    assert_ne!(t1, t2);
}

#[test]
fn missing_side_defaults_blufor() {
    assert_eq!(side_rgba(""), SIDE_BLUFOR_RGBA);
    assert_eq!(side_rgba("UNKNOWN"), SIDE_BLUFOR_RGBA);
    let xy = [0.0_f32, 0.0, 1.0, 1.0];

    let bytes = pack_rings(&xy, &[false, false], &["OPFOR"]);
    let t0 = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let t1 = u32::from_le_bytes(bytes[20 + 16..20 + 20].try_into().unwrap());
    assert_eq!(t0, pack_rgba_u32(SIDE_OPFOR_RGBA));
    assert_eq!(t1, pack_rgba_u32(SIDE_BLUFOR_RGBA));
    let bytes2 = pack_rings(&xy, &[false, false], &["", "UNKNOWN"]);
    assert_eq!(
        u32::from_le_bytes(bytes2[16..20].try_into().unwrap()),
        pack_rgba_u32(SIDE_BLUFOR_RGBA)
    );
    assert_eq!(
        u32::from_le_bytes(bytes2[20 + 16..20 + 20].try_into().unwrap()),
        pack_rgba_u32(SIDE_BLUFOR_RGBA)
    );
}

#[test]
fn cluster_gate_truth_table() {
    assert!(!cluster_mode(0, -6.0));
    assert!(!cluster_mode(500, -6.0));
    assert!(!cluster_mode(501, -3.9));
    assert!(cluster_mode(501, -4.0));
    assert!(cluster_mode(10_000, -6.0));
    assert!(!cluster_mode(10_000, -2.0));
}

#[test]
fn cluster_disc_size_formula() {
    assert!((cluster_disc_size_px(1) - 22.0).abs() < 1e-5);

    assert!((cluster_disc_size_px(1000) - 48.0).abs() < 1e-5);
}

#[test]
fn drag_delta_math() {
    let (x, y) = drag_projected(100.0, 200.0, 3.5, -1.25);
    assert!((x - 103.5).abs() < 1e-12);
    assert!((y - 198.75).abs() < 1e-12);
}

#[test]
fn px_to_m_at_default_zoom() {
    assert!((px_to_m_at_zoom(-2.0) - 4.0).abs() < 1e-6);
    assert!((px_to_m_at_zoom(0.0) - 1.0).abs() < 1e-6);
    assert!((px_to_m_at_zoom(3.0) - 0.125).abs() < 1e-6);
}

#[test]
fn pack_cluster_instances_count() {
    let xs = [10.0, 20.0];
    let ys = [30.0, 40.0];
    let counts = [5u32, 100];
    let b = pack_cluster_instances(&xs, &ys, &counts);
    assert_eq!(b.len(), 2 * SLOT_ICON_STRIDE);
    assert_eq!(
        u16::from_le_bytes(b[14..16].try_into().unwrap()),
        SLOT_GLYPH_DISC
    );
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
fn pack_drag_overlay_rows() {
    let ids = vec!["a".into(), "b".into()];
    let xy = [1.0_f32, 2.0, 3.0, 4.0];
    let drag = vec!["b".into()];
    let (bytes, rows) = pack_drag_overlay(&drag, &ids, &xy);
    assert_eq!(rows, vec![1]);
    assert_eq!(bytes.len(), SLOT_ICON_STRIDE);
    let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
    assert!((x - 3.0).abs() < 1e-6);
}

#[test]
fn vehicle_drag_preview_offsets_only_the_dragged_vehicles_of_a_mixed_selection() {
    let points = vec![
        ("v-parked".to_string(), 10.0, 20.0),
        ("v-dragged-a".to_string(), 30.0, 40.0),
        ("v-dragged-b".to_string(), 50.0, 60.0),
    ];

    let drag: Vec<String> = ["slot-7", "v-dragged-b", "slot-9", "v-dragged-a"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    let lane = pack_vehicle_drag_preview(&drag, &points, 7.5, -3.25);

    assert_eq!(
        lane.len(),
        points.len() * 2,
        "every placed vehicle must stay in the dense lane — a dropped row VANISHES mid-drag"
    );
    assert_eq!(
        lane,
        vec![10.0_f32, 20.0, 37.5, 36.75, 57.5, 56.75],
        "v-parked stands still; both dragged vehicles move by exactly (+7.5, -3.25)"
    );
}

#[test]
fn vehicle_drag_preview_leaves_the_lane_authored_when_no_vehicle_is_dragged() {
    let points = vec![
        ("v0".to_string(), 10.0, 20.0),
        ("v1".to_string(), 30.0, 40.0),
    ];
    let slots_only: Vec<String> = vec!["slot-1".to_string(), "slot-2".to_string()];
    assert_eq!(
        pack_vehicle_drag_preview(&slots_only, &points, 7.5, -3.25),
        vec![10.0_f32, 20.0, 30.0, 40.0],
        "a slots-only drag must not smear the vehicle lane"
    );
    assert_eq!(
        pack_vehicle_drag_preview(&[], &points, 7.5, -3.25),
        vec![10.0_f32, 20.0, 30.0, 40.0],
        "empty drag = the identity re-pack the cancel path restores with"
    );
    assert!(
        pack_vehicle_drag_preview(&slots_only, &[], 1.0, 1.0).is_empty(),
        "no placed vehicles ⇒ empty lane (vehicles_bind drops the lane)"
    );
}

#[test]
fn slot_atlas_shape_and_uv() {
    let a = build_slot_atlas();
    assert_eq!(a.rgba.len(), 128 * 64 * 4);
    assert_eq!((a.width, a.height), (128, 64));
    assert_eq!(a.uv, [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 1.0, 1.0]);
}

#[test]
fn every_seeded_kit_maps_and_unknown_defaults() {
    let seeded: [(&str, UnitRoleClass); 12] = [
        ("kit:us_rifleman", UnitRoleClass::Rifleman),
        ("kit:us_sl", UnitRoleClass::Leader),
        ("kit:us_tl", UnitRoleClass::Leader),
        ("kit:us_ar", UnitRoleClass::MachineGun),
        ("kit:us_medic", UnitRoleClass::Medic),
        ("kit:fia_rifleman", UnitRoleClass::Rifleman),
        ("kit:fia_sl", UnitRoleClass::Leader),
        ("kit:fia_medic", UnitRoleClass::Medic),
        ("kit:sov_rifleman", UnitRoleClass::Rifleman),
        ("kit:sov_sl", UnitRoleClass::Leader),
        ("kit:sov_ar", UnitRoleClass::MachineGun),
        ("kit:civ_generic", UnitRoleClass::Rifleman),
    ];
    for (alias, want) in seeded {
        assert_eq!(unit_role_class(alias), want, "seeded kit {alias}");

        let bare = alias.strip_prefix("kit:").unwrap();
        assert_eq!(unit_role_class(bare), want, "bare {bare}");
    }

    for unknown in [
        "",
        "   ",
        "kit:xx_unheard_of",
        "Sapper",
        "not-a-role",
        "kit:",
    ] {
        assert_eq!(
            unit_role_class(unknown),
            UnitRoleClass::Rifleman,
            "unknown {unknown:?} must default"
        );
    }
}

#[test]
fn authored_role_strings_and_token_boundaries() {
    assert_eq!(unit_role_class("Squad Leader"), UnitRoleClass::Leader);
    assert_eq!(unit_role_class("Team Leader"), UnitRoleClass::Leader);
    assert_eq!(unit_role_class("Medic"), UnitRoleClass::Medic);
    assert_eq!(unit_role_class("AT Gunner"), UnitRoleClass::AntiTank);
    assert_eq!(
        unit_role_class("anti-tank rifleman"),
        UnitRoleClass::AntiTank
    );
    assert_eq!(unit_role_class("Machine Gunner"), UnitRoleClass::MachineGun);
    assert_eq!(
        unit_role_class("Automatic Rifleman"),
        UnitRoleClass::MachineGun
    );
    assert_eq!(unit_role_class("Rifleman"), UnitRoleClass::Rifleman);

    assert_eq!(unit_role_class("Medic Team Leader"), UnitRoleClass::Leader);
    assert_eq!(unit_role_class("AT Medic"), UnitRoleClass::Medic);
    assert_eq!(unit_role_class("AT Gunner MG"), UnitRoleClass::AntiTank);

    assert_eq!(unit_role_class("Attack Rifleman"), UnitRoleClass::Rifleman);
    assert_eq!(unit_role_class("Station Guard"), UnitRoleClass::Rifleman);
    assert_eq!(unit_role_class("Combat Engineer"), UnitRoleClass::Rifleman);

    assert_eq!(unit_role_class("Marksman"), UnitRoleClass::Rifleman);
    assert_eq!(unit_role_class("Sharpshooter"), UnitRoleClass::Rifleman);
}

#[test]
fn seeded_vehicles_map_to_silhouette_kinds() {
    for a in [
        "veh:us_m1025",
        "M1025 Humvee",
        "{4A71F755A4513227}Prefabs/Vehicles/Wheeled/M998/M1025.et",
        "M998 Humvee",
    ] {
        assert_eq!(vehicle_kind_for_alias(a), VehicleKind::WheeledLight, "{a}");
    }
    for a in [
        "veh:us_m923a1",
        "M923A1",
        "Prefabs/Vehicles/Wheeled/M923A1/M923A1.et",
        "Ural cargo truck",
    ] {
        assert_eq!(vehicle_kind_for_alias(a), VehicleKind::Truck, "{a}");
    }
    for a in ["veh:us_m113", "M113A3", "BTR-70", "BMP-1", "tracked ifv"] {
        assert_eq!(vehicle_kind_for_alias(a), VehicleKind::Apc, "{a}");
    }

    assert_eq!(vehicle_kind_for_alias(""), VehicleKind::WheeledLight);
    assert_eq!(
        vehicle_kind_for_alias("veh:unheard_of"),
        VehicleKind::WheeledLight
    );
}

#[test]
fn slot_lane_carries_yaw_and_the_sign_is_the_compass_convention() {
    let yaw_of = |bytes: &[u8]| i16::from_le_bytes(bytes[12..14].try_into().unwrap());
    let xy = [100.0_f32, 200.0];
    let roles = vec!["kit:us_rifleman".to_string()];
    let pack = |h: f32| pack_slot_symbology(&xy, &[false], &[], &roles, &[h], 1.0, 11);

    let h90 = pack(90.0);
    let h180 = pack(180.0);
    assert_ne!(
        h90, h180,
        "heading 90 and 180 must not pack identically — that IS the T-832 defect"
    );

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
fn screen_yaw_sign_and_degenerates() {
    assert!((screen_yaw_for_heading_deg(90.0) - -90.0).abs() < 1e-12);
    assert!((screen_yaw_for_heading_deg(270.0) - -270.0).abs() < 1e-12);
    assert!((screen_yaw_for_heading_deg(0.0)).abs() < 1e-12);
    assert!(
        !screen_yaw_for_heading_deg(0.0).is_sign_negative(),
        "0.0 must not become -0.0"
    );
    assert!((screen_yaw_for_heading_deg(f64::NAN)).abs() < 1e-12);
    assert!((screen_yaw_for_heading_deg(f64::INFINITY)).abs() < 1e-12);

    assert!((screen_yaw_for_heading_deg(359.0) - -359.0).abs() < 1e-12);

    assert_eq!(yaw_to_snorm16(-270.0), yaw_to_snorm16(90.0));
    assert_eq!(yaw_to_snorm16(180.0), 32767);
    assert_eq!(
        yaw_to_snorm16(-180.0),
        32767,
        "-180 folds up into (-180, 180]"
    );
    assert_eq!(yaw_to_snorm16(540.0), yaw_to_snorm16(180.0));
    assert_eq!(yaw_to_snorm16(-359.0), yaw_to_snorm16(1.0));
}

#[test]
fn every_heading_of_the_compass_gets_its_own_facing() {
    let drawn_bearing = |heading: f64| {
        let snorm = yaw_to_snorm16(screen_yaw_for_heading_deg(heading));
        let screen_deg = f64::from(snorm) / 32767.0 * 180.0;
        (-screen_deg).rem_euclid(360.0)
    };
    let encode = |heading: f64| yaw_to_snorm16(screen_yaw_for_heading_deg(heading));

    let cardinals = [encode(0.0), encode(90.0), encode(180.0), encode(270.0)];
    for (i, a) in cardinals.iter().enumerate() {
        for (j, b) in cardinals.iter().enumerate() {
            assert!(
                i == j || a != b,
                "cardinals {i} and {j} share encoding {a} — the compass has collapsed"
            );
        }
    }
    assert_eq!(
        cardinals[3],
        encode(-90.0),
        "270 IS -90, not the clamp floor"
    );

    assert_eq!(encode(359.0), encode(-1.0));
    assert!(
        (drawn_bearing(359.0) - 359.0).abs() < 0.02,
        "359 drew {}",
        drawn_bearing(359.0)
    );

    let mut prev = drawn_bearing(0.0);
    for h in 1..=360 {
        let cur = drawn_bearing(f64::from(h));
        let step = (cur - prev).rem_euclid(360.0);
        assert!(
            step > 0.0,
            "heading {h} did not move the glyph — plateau at bearing {prev} (this is T-808)"
        );
        assert!(
            step < 2.0,
            "heading {h} jumped {step}° — the fold is not continuous"
        );
        assert!(
            (cur - f64::from(h % 360)).abs() < 0.02 || (cur - f64::from(h % 360)).abs() > 359.9,
            "heading {h} drew bearing {cur}"
        );
        prev = cur;
    }
}

#[cfg(feature = "streaming")]
#[test]
fn yaw_encoders_agree() {
    for i in -400..=400 {
        let deg = f64::from(i) * 0.9375;
        assert_eq!(
            yaw_to_snorm16(deg),
            crate::overlay::symbology::labels::glyph_math::yaw_to_snorm16(deg),
            "yaw encoders diverged at {deg}"
        );
    }
    for deg in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0] {
        assert_eq!(
            yaw_to_snorm16(deg),
            crate::overlay::symbology::labels::glyph_math::yaw_to_snorm16(deg)
        );
    }

    for deg in [0.0, 45.0, 90.0, 180.0, 359.5] {
        assert!(
            (screen_yaw_for_heading_deg(deg)
                - crate::overlay::symbology::labels::glyph_math::deck_angle_for_rotation_deg(deg))
            .abs()
                < 1e-12,
            "screen-angle convention diverged at {deg}"
        );
    }
}

#[test]
fn extend_atlas_copies_the_base_verbatim_and_bases_after_it() {
    let base = build_slot_atlas();
    let a =
        extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height).expect("well-formed");
    let (rgba, w, h, uv, base_cells) = (&a.rgba, a.width, a.height, &a.uv, a.base_cells);
    assert_eq!(base_cells, 2, "build_slot_atlas is a 2-cell strip");
    let n = 2 + SYMBOLOGY_CELL_COUNT;
    #[allow(clippy::cast_possible_truncation)]
    {
        assert_eq!((w, h), ((n * 64) as u32, 64));
    }
    assert_eq!(rgba.len(), n * 64 * 64 * 4);
    assert_eq!(uv.len(), n * 4);

    for y in 0..64usize {
        let src = y * (base.width as usize) * 4;
        let dst = y * (w as usize) * 4;
        let len = base.width as usize * 4;
        assert_eq!(
            &rgba[dst..dst + len],
            &base.rgba[src..src + len],
            "base atlas row {y} was not copied verbatim"
        );
    }

    #[allow(clippy::cast_precision_loss)]
    {
        assert!((uv[0]).abs() < 1e-9);
        assert!((uv[2] - 1.0 / n as f32).abs() < 1e-6);
    }

    let wide = vec![0u8; 3 * 64 * 64 * 4];
    let b3 = extend_atlas_with_unit_glyphs(&wide, 3 * 64, 64).expect("3-cell");
    assert_eq!(b3.base_cells, 3);
}

#[test]
fn extend_atlas_refuses_a_strip_it_cannot_read() {
    assert!(extend_atlas_with_unit_glyphs(&[], 0, 64).is_none(), "empty");
    assert!(
        extend_atlas_with_unit_glyphs(&vec![0u8; 128 * 32 * 4], 128, 32).is_none(),
        "wrong cell height"
    );
    assert!(
        extend_atlas_with_unit_glyphs(&vec![0u8; 100 * 64 * 4], 100, 64).is_none(),
        "width not a whole number of cells"
    );
    assert!(
        extend_atlas_with_unit_glyphs(&[0u8; 10], 128, 64).is_none(),
        "rgba length disagrees with w·h·4"
    );
}

#[test]
fn symbology_cells_are_pairwise_distinct_shapes() {
    let base = build_slot_atlas();
    let a = extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height).expect("atlas");
    let (rgba, base_cells) = (&a.rgba, a.base_cells);
    let w = a.width as usize;
    let cell_alpha = |c: usize| -> Vec<u8> {
        let mut v = Vec::with_capacity(64 * 64);
        for y in 0..64 {
            for x in 0..64 {
                v.push(rgba[(y * w + c * 64 + x) * 4 + 3]);
            }
        }
        v
    };
    let named: Vec<(&str, usize)> = vec![
        ("marker-ring", 0),
        ("marker-disc", 1),
        (
            "unit-rifleman",
            base_cells as usize + UNIT_CELL_BASE as usize,
        ),
        (
            "unit-leader",
            base_cells as usize + UNIT_CELL_BASE as usize + 1,
        ),
        (
            "unit-medic",
            base_cells as usize + UNIT_CELL_BASE as usize + 2,
        ),
        ("unit-at", base_cells as usize + UNIT_CELL_BASE as usize + 3),
        ("unit-mg", base_cells as usize + UNIT_CELL_BASE as usize + 4),
        (
            "unit-rifleman-sel",
            base_cells as usize + UNIT_SELECTED_CELL_BASE as usize,
        ),
        (
            "veh-wheeled",
            base_cells as usize + VEHICLE_CELL_BASE as usize,
        ),
        (
            "veh-truck",
            base_cells as usize + VEHICLE_CELL_BASE as usize + 1,
        ),
        (
            "veh-apc",
            base_cells as usize + VEHICLE_CELL_BASE as usize + 2,
        ),
        ("comment", base_cells as usize + COMMENT_CELL as usize),
        (
            "comment-sel",
            base_cells as usize + COMMENT_SELECTED_CELL as usize,
        ),
    ];
    let shots: Vec<(&str, Vec<u8>, u32)> = named
        .iter()
        .map(|(n, c)| {
            let a = cell_alpha(*c);
            let ink: u32 = a.iter().map(|&p| u32::from(p)).sum();
            (*n, a, ink)
        })
        .collect();
    for (n, _, ink) in &shots {
        assert!(*ink > 0, "cell {n} is BLANK — a glyph that draws nothing");
    }
    for i in 0..shots.len() {
        for j in (i + 1)..shots.len() {
            assert_ne!(
                shots[i].1, shots[j].1,
                "{} and {} have identical alpha bitmaps",
                shots[i].0, shots[j].0
            );
            assert_ne!(
                shots[i].2, shots[j].2,
                "{} and {} have identical total ink",
                shots[i].0, shots[j].0
            );
        }
    }
}

#[test]
fn role_knockouts_differ_inside_the_disc() {
    let base = build_slot_atlas();
    let a = extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height).expect("atlas");
    let (rgba, bc) = (&a.rgba, a.base_cells);
    let w = a.width as usize;
    let interior = |c: usize| -> Vec<u8> {
        let mut v = Vec::new();
        for y in 18..46 {
            for x in 18..46 {
                let dx = x as f64 + 0.5 - 32.0;
                let dy = y as f64 + 0.5 - 32.0;
                if dx.hypot(dy) <= 14.0 {
                    v.push(rgba[(y * w + c * 64 + x) * 4 + 3]);
                }
            }
        }
        v
    };
    let cell = |cls: UnitRoleClass| bc as usize + UNIT_CELL_BASE as usize + cls as usize;
    let rifleman = interior(cell(UnitRoleClass::Rifleman));
    let leader = interior(cell(UnitRoleClass::Leader));
    let medic = interior(cell(UnitRoleClass::Medic));
    let at = interior(cell(UnitRoleClass::AntiTank));
    let mg = interior(cell(UnitRoleClass::MachineGun));
    assert!(
        rifleman.iter().all(|&p| p == 255),
        "the default role must be a SOLID disc (no knockout)"
    );
    for (n, s) in [
        ("leader", &leader),
        ("medic", &medic),
        ("at", &at),
        ("mg", &mg),
    ] {
        assert!(s.contains(&0), "{n} must knock a real hole in the body");
    }
    let all = [
        ("rifleman", &rifleman),
        ("leader", &leader),
        ("medic", &medic),
        ("at", &at),
        ("mg", &mg),
    ];
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(
                all[i].1, all[j].1,
                "{} and {} share an interior",
                all[i].0, all[j].0
            );
        }
    }
}

#[test]
fn unit_cell_is_north_asymmetric_so_yaw_is_visible() {
    let base = build_slot_atlas();
    let a = extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height).expect("atlas");
    let (rgba, bc) = (&a.rgba, a.base_cells);
    let w = a.width as usize;
    let c = bc as usize + UNIT_CELL_BASE as usize;
    let a = |x: usize, y: usize| rgba[(y * w + c * 64 + x) * 4 + 3];

    assert!(a(32, 6) > 0, "the facing point must reach the north edge");
    assert_eq!(a(32, 57), 0, "nothing may hang off the south edge");

    let north: Vec<u8> = (0..32)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .map(|(x, y)| a(x, y))
        .collect();
    let south: Vec<u8> = (32..64)
        .rev()
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .map(|(x, y)| a(x, y))
        .collect();
    assert_ne!(
        north, south,
        "the cell is mirror-symmetric about its east-west axis, so a 180° yaw would be invisible"
    );
}

#[test]
fn symbology_degrades_to_dots_past_the_stated_m_per_px() {
    assert!(symbology_visible(1.0), "1.0 m/px is an acceptance zoom");
    assert!(symbology_visible(4.0), "4.0 m/px is an acceptance zoom");
    assert!(symbology_visible(SYMBOLOGY_MAX_M_PER_PX));
    assert!(!symbology_visible(SYMBOLOGY_MAX_M_PER_PX + 0.001));
    assert!(!symbology_visible(32.0));

    assert!(!symbology_visible(0.0));
    assert!(!symbology_visible(-1.0));
    assert!(!symbology_visible(f32::NAN));

    let xy = [10.0_f32, 20.0];
    let roles = vec!["kit:us_medic".to_string()];
    let glyph_of = |b: &[u8]| u16::from_le_bytes(b[14..16].try_into().unwrap());
    let size_of = |b: &[u8]| f32::from_le_bytes(b[8..12].try_into().unwrap());

    let near = pack_slot_symbology(&xy, &[false], &[], &roles, &[90.0], 4.0, 11);
    assert_eq!(glyph_of(&near), 11 + UnitRoleClass::Medic as u16);
    assert!((size_of(&near) - SLOT_UNIT_PX).abs() < 1e-6);

    let far = pack_slot_symbology(&xy, &[false], &[], &roles, &[90.0], 16.0, 11);
    assert_eq!(glyph_of(&far), SLOT_GLYPH_DISC, "degraded to a plain dot");
    assert!((size_of(&far) - SLOT_RING_PX).abs() < 1e-6);
    assert_eq!(
        i16::from_le_bytes(far[12..14].try_into().unwrap()),
        0,
        "a dot claims no heading"
    );

    assert_eq!(
        u32::from_le_bytes(far[16..20].try_into().unwrap()),
        pack_rgba_u32(SIDE_BLUFOR_RGBA)
    );
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

#[test]
fn comments_are_neutral_bubbles_with_selection_on_top() {
    let xy = [1.0_f32, 2.0, 3.0, 4.0];
    let b = pack_comment_instances(&xy, &[false, true], 1.0, 11);
    let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];
    let glyph = |i: usize| u16::from_le_bytes(row(i)[14..16].try_into().unwrap());
    let tint = |i: usize| u32::from_le_bytes(row(i)[16..20].try_into().unwrap());

    assert_eq!(glyph(0), 11 + COMMENT_CELL);
    assert_eq!(tint(0), pack_rgba_u32(COMMENT_NOTE_RGBA));
    assert_ne!(
        tint(0),
        pack_rgba_u32(SLOT_SELECTED_RGBA),
        "an idle comment must NOT wear the selection colour (the T-796 defect)"
    );
    assert_ne!(
        glyph(0),
        SLOT_GLYPH_RING,
        "an idle comment must NOT wear the slot ring glyph (the T-796 defect)"
    );
    assert_eq!(glyph(1), 11 + COMMENT_SELECTED_CELL);
    assert_eq!(tint(1), pack_rgba_u32(SLOT_SELECTED_RGBA));

    let none = pack_comment_instances(&xy, &[], 1.0, 11);
    assert_eq!(
        u32::from_le_bytes(none[16..20].try_into().unwrap()),
        pack_rgba_u32(COMMENT_NOTE_RGBA)
    );
    assert_eq!(none.len(), 2 * SLOT_ICON_STRIDE);
}
