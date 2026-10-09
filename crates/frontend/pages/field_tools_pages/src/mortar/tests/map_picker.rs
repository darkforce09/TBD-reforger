//! The guards on the map picker: placing positions, grabbing markers, the crest profile, and the
//! fire-mission overlay buffers.

use super::marks::{DISPERSION_FILL_ALPHA, glyph_icon, lane_uploads, overlay_scene};
use super::picking::{
    Placement, apply_placement, marker_near, placed_markers, placement_options, valid_placement,
};
use super::profile::{PROFILE_STEP_M, coverage_manifest, lead_gun_profile};
use crate::mortar::inputs::positions::{HeightChoice, MortarTerrain};
use crate::mortar::inputs::weapon_and_shell::{ArmamentSelection, ChargeChoice};
use crate::mortar::inputs::wind::WindDraft;
use crate::mortar::solve_bridge::{MissionDrafts, solve_mission};
use crate::mortar::test_mission::{catalog, gun, manual, solved_mission};
use overlay_instances::fire_mission_marks::{
    DISPERSION_RING_VERTICES, FireMissionGlyph, GUN_COLOR, TARGET_COLOR,
};

#[test]
fn a_placement_round_trips_through_its_option_value() {
    for p in [Placement::Target, Placement::Gun(0), Placement::Gun(7)] {
        assert_eq!(Placement::from_option_value(&p.option_value()), Some(p));
    }
    for junk in ["", "gun-", "gun-x", "Target", "gun--1"] {
        assert_eq!(Placement::from_option_value(junk), None, "{junk:?}");
    }
    let guns = [gun(3, "Gun 1", "", ""), gun(5, "  ", "", "")];
    let labels: Vec<String> = placement_options(&guns)
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    assert_eq!(labels, ["Target", "Gun 1", "Unnamed gun"]);
}

/// A click writes a 10-figure grid into the one position placed, and nothing else.
#[test]
fn a_click_writes_only_the_placed_position_as_a_ten_figure_grid() {
    let mut target = manual("064 129", "40");
    let mut guns = vec![
        gun(0, "Gun 1", "055 125", "25"),
        gun(4, "Gun 2", "056 124", "30"),
    ];
    assert!(apply_placement(
        &mut target,
        &mut guns,
        Placement::Gun(4),
        5_612.3,
        12_401.9
    ));
    assert_eq!(guns[1].position.grid, "05612 12401");
    assert_eq!(guns[1].position.height_choice, HeightChoice::Manual);
    assert_eq!(
        guns[1].position.manual_height, "30",
        "the height is left as typed"
    );
    assert_eq!(
        (target.grid.as_str(), guns[0].position.grid.as_str()),
        ("064 129", "055 125")
    );

    assert!(apply_placement(
        &mut target,
        &mut guns,
        Placement::Target,
        6_400.0,
        12_900.0
    ));
    assert_eq!(target.grid, "06400 12900");

    let before = (target.clone(), guns.clone());
    assert!(!apply_placement(
        &mut target,
        &mut guns,
        Placement::Gun(9),
        1.0,
        1.0
    ));
    assert_eq!(
        (target, guns),
        before,
        "a gun no longer in the battery is never written"
    );
}

#[test]
fn a_placement_of_a_removed_gun_falls_back_to_the_target() {
    let guns = [gun(0, "Gun 1", "", "")];
    assert_eq!(valid_placement(Placement::Gun(0), &guns), Placement::Gun(0));
    assert_eq!(valid_placement(Placement::Gun(2), &guns), Placement::Target);
    assert_eq!(valid_placement(Placement::Target, &[]), Placement::Target);
}

/// A press grabs the nearest placed marker within the radius, and nothing farther.
#[test]
fn a_press_grabs_the_nearest_marker_within_the_hit_radius() {
    let target = manual("064 129", "");
    let guns = [
        gun(0, "Gun 1", "055 125", ""),
        gun(1, "Gun 2", "not a grid", ""),
    ];
    let markers = placed_markers(&target, &guns);
    assert_eq!(
        markers,
        [
            (Placement::Gun(0), [5_550.0, 12_550.0]),
            (Placement::Target, [6_450.0, 12_950.0])
        ],
        "an unparsed grid places no marker"
    );
    assert_eq!(
        marker_near(&markers, [5_560.0, 12_550.0], 20.0),
        Some(Placement::Gun(0))
    );
    assert_eq!(marker_near(&markers, [5_600.0, 12_550.0], 20.0), None);
    assert_eq!(marker_near(&markers, [5_550.0, 12_550.0], 0.0), None);
    assert_eq!(marker_near(&markers, [5_550.0, 12_550.0], f64::NAN), None);
    let close = [
        (Placement::Gun(0), [0.0, 0.0]),
        (Placement::Target, [10.0, 0.0]),
    ];
    assert_eq!(
        marker_near(&close, [6.0, 0.0], 50.0),
        Some(Placement::Target)
    );
}

#[test]
fn the_profile_samples_the_lead_gun_line_at_the_native_resolution() {
    let target = manual("064 119", "40");
    let guns = [
        gun(0, "Gun 1", "055 115", "25"),
        gun(1, "Gun 2", "056 114", "30"),
    ];
    let profile = lead_gun_profile(MortarTerrain::Everon, &target, &guns, |x, _| {
        Some(x / 100.0)
    })
    .expect("a loaded terrain yields a profile");
    let distance = (900.0_f64).hypot(400.0);
    let first = profile.samples.first().unwrap();
    let last = profile.samples.last().unwrap();
    assert_eq!(
        (first.downrange_m, first.height_m),
        (0.0, 55.5),
        "it starts at the lead gun"
    );
    assert!(
        (last.downrange_m - distance).abs() < 1e-6,
        "it ends at the target"
    );
    assert!((last.height_m - 64.5).abs() < 1e-9);
    for pair in profile.samples.windows(2) {
        let step = pair[1].downrange_m - pair[0].downrange_m;
        assert!(step > 0.0 && step <= PROFILE_STEP_M + 1e-9, "step {step}");
    }
}

#[test]
fn no_profile_without_an_elevation_model_heights_or_grids() {
    let target = manual("064 129", "40");
    let guns = [gun(0, "Gun 1", "055 125", "25")];
    assert!(coverage_manifest(MortarTerrain::Arland).is_none());
    assert!(lead_gun_profile(MortarTerrain::Arland, &target, &guns, |_, _| Some(1.0)).is_none());
    assert!(lead_gun_profile(MortarTerrain::Everon, &target, &guns, |_, _| None).is_none());
    assert!(
        lead_gun_profile(MortarTerrain::Everon, &target, &guns, |_, _| Some(f64::NAN)).is_none(),
        "a non-finite height is no sample"
    );
    assert!(
        lead_gun_profile(MortarTerrain::Everon, &manual("bad", ""), &guns, |_, _| {
            Some(1.0)
        })
        .is_none()
    );
    assert!(lead_gun_profile(MortarTerrain::Everon, &target, &[], |_, _| Some(1.0)).is_none());
}

/// The profile reaches the solution: flat low ground clears, a ridge higher than the apex blocks.
#[test]
fn the_crest_check_clears_low_ground_and_warns_over_a_ridge() {
    let chosen = ArmamentSelection {
        weapon_id: "m252".into(),
        shell_id: "m853a1".into(),
        charge: ChargeChoice::Recommended,
    };
    let target = manual("064 129", "40");
    let guns = [gun(0, "Gun 1", "055 125", "25")];
    let wind = WindDraft::default();
    let crest_over = |ground: &dyn Fn(f64, f64) -> Option<f64>| {
        let profile = lead_gun_profile(MortarTerrain::Everon, &target, &guns, ground);
        let drafts = MissionDrafts {
            selection: &chosen,
            terrain: MortarTerrain::Everon,
            target: &target,
            guns: &guns,
            wind: &wind,
            burst_height: "",
            crest_profile: profile.as_ref(),
        };
        solve_mission(&catalog(), drafts, |_, _| None)
            .expect("the mission solves")
            .solution
            .crest
            .expect("a profile yields a crest clearance")
    };
    let flat = crest_over(&|_, _| Some(0.0));
    assert_eq!(flat.first_blocking_downrange_m, None);
    assert!(flat.min_clearance_m > 0.0);
    let ridge = crest_over(&|x, _| {
        Some(if (5_900.0..6_000.0).contains(&x) {
            5_000.0
        } else {
            0.0
        })
    });
    let blocking = ridge.first_blocking_downrange_m.expect("the ridge blocks");
    assert!(
        blocking > 300.0 && blocking < 600.0,
        "blocked at {blocking}"
    );
    assert!(ridge.min_clearance_m < 0.0);
}

/// The buffers are in world metres, aligned glyph for glyph, with one fan per ellipse.
#[test]
fn the_lane_uploads_are_world_metres_aligned_glyph_for_glyph() {
    let solved = solved_mission();
    let target = manual("064 129", "40");
    let guns = [
        gun(0, "Gun 1", "055 125", "25"),
        gun(1, "Gun 2", "056 124", "30"),
    ];
    let scene = overlay_scene(&target, &guns, Some(&solved));
    let up = lane_uploads(&scene, 10.0);
    assert_eq!(
        up.glyph_xy,
        [5_550.0, 12_550.0, 5_650.0, 12_450.0, 6_450.0, 12_950.0]
    );
    assert_eq!(up.glyph_captions, scene.captions);
    assert_eq!(
        up.glyph_icons,
        [
            glyph_icon(FireMissionGlyph::Gun),
            glyph_icon(FireMissionGlyph::Gun),
            glyph_icon(FireMissionGlyph::Target)
        ]
    );
    let tint = |c: [f32; 4]| c.map(|v| (v * 255.0).round() as u8);
    assert_eq!(&up.glyph_rgba[0..4], &tint(GUN_COLOR));
    assert_eq!(&up.glyph_rgba[8..12], &tint(TARGET_COLOR));
    let ellipses = scene.plot.dispersion.len() as u32;
    assert_eq!(
        up.line_segments,
        2 + ellipses * DISPERSION_RING_VERTICES as u32
    );
    assert_eq!(up.line_packed.len() as u32, up.line_segments * 2 * 6);
    let first_line_start = [up.line_packed[0], up.line_packed[1]];
    let trimmed = (first_line_start[0] - 5_550.0).hypot(first_line_start[1] - 12_550.0);
    assert!(
        (trimmed - 10.0).abs() < 0.01,
        "a line stops at the glyph edge: {trimmed}"
    );
    assert_eq!(up.fill_count, ellipses);
    assert_eq!(
        up.fill_indices.len(),
        (ellipses as usize) * (DISPERSION_RING_VERTICES - 2) * 3
    );
    assert_eq!(up.fill_colors.len(), up.fill_positions.len() * 2);
    assert!(
        up.fill_colors
            .chunks(4)
            .all(|c| c[3] == DISPERSION_FILL_ALPHA)
    );
    assert!(
        up.fill_indices
            .iter()
            .all(|&i| (i as usize) < up.fill_positions.len() / 2)
    );
}
