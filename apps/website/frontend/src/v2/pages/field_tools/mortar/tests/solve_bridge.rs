//! Unit tests for the solve bridge: the drafts mapped onto the engine's fire-mission inputs, the
//! solution being the engine's own, every problem reported at once, gun independence and order,
//! the wind hand-over, and the wording of each charge row.

use super::*;
use website_map_engine::data::scenario::ballistics::angular_units::MilsConvention;
use website_map_engine::data::scenario::ballistics::crest_clearance::TerrainSample;
use website_map_engine::data::scenario::ballistics::fire_mission::{
    FireMissionGunPosition, FireMissionPoint, FireMissionWind, HeightSource,
};
use website_map_engine::data::scenario::ballistics::solution_wording::{
    deflection_text, range_correction_text,
};
use website_map_engine::data::scenario::ballistics::solver::SolutionRefusal;

fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("mortar_test_catalog.json"))
        .expect("the test catalog decodes")
}

fn manual(grid: &str, height: &str) -> PositionDraft {
    PositionDraft {
        grid: grid.into(),
        height_choice: super::super::inputs::positions::HeightChoice::Manual,
        manual_height: height.into(),
    }
}

fn gun(key: u32, label: &str, grid: &str, height: &str) -> GunDraft {
    GunDraft {
        key,
        label: label.into(),
        position: manual(grid, height),
    }
}

fn selection(weapon: &str, shell: &str, charge: ChargeChoice) -> ArmamentSelection {
    ArmamentSelection {
        weapon_id: weapon.into(),
        shell_id: shell.into(),
        charge,
    }
}

/// Drafts with no wind, no burst height and no crest profile.
fn drafts<'d>(
    chosen: &'d ArmamentSelection,
    target: &'d PositionDraft,
    guns: &'d [GunDraft],
    wind: &'d WindDraft,
) -> MissionDrafts<'d> {
    MissionDrafts {
        selection: chosen,
        terrain: MortarTerrain::Everon,
        target,
        guns,
        wind,
        burst_height: "",
        crest_profile: None,
    }
}

#[test]
fn the_drafts_map_onto_the_engine_inputs_pinned_to_the_catalog() {
    let catalog = catalog();
    let chosen = selection("m252", "m853a1", ChargeChoice::Rings(3));
    let target = PositionDraft {
        grid: " 064 129 ".into(),
        ..PositionDraft::default()
    };
    let guns = [gun(0, " Gun 1 ", "0555 1255", "21.5")];
    let wind = WindDraft {
        speed_m_s: "3".into(),
        from_deg: "-45".into(),
    };
    let profile = TerrainProfile {
        samples: vec![
            TerrainSample {
                downrange_m: 0.0,
                height_m: 21.5,
            },
            TerrainSample {
                downrange_m: 500.0,
                height_m: 80.0,
            },
        ],
    };
    let drafts = MissionDrafts {
        burst_height: "600",
        crest_profile: Some(&profile),
        ..drafts(&chosen, &target, &guns, &wind)
    };
    let inputs = mission_inputs(&catalog, drafts, |_, _| Some(48.0)).expect("maps");
    assert_eq!(
        inputs,
        FireMissionInputs {
            catalog_id: "mortar-page-test".into(),
            catalog_version: 2,
            weapon_id: "m252".into(),
            shell_id: "m853a1".into(),
            charge_rings: Some(3),
            target: FireMissionPoint {
                x: 6_450.0,
                y: 12_950.0,
                height_m: 48.0,
                height_source: HeightSource::Dem,
            },
            guns: vec![FireMissionGunPosition {
                label: "Gun 1".into(),
                x: 5_555.0,
                y: 12_555.0,
                height_m: 21.5,
                height_source: HeightSource::Manual,
            }],
            wind: Some(FireMissionWind {
                speed_m_s: 3.0,
                from_deg: 315.0,
            }),
            burst_height_m: Some(600.0),
            crest_profile: Some(profile.clone()),
        }
    );
}

#[test]
fn the_solution_is_the_engine_fire_mission_solution_byte_for_byte() {
    let catalog = catalog();
    let chosen = selection("m252", "m853a1", ChargeChoice::Recommended);
    let target = manual("064 129", "40");
    let guns = [
        gun(0, "Gun 1", "055 125", "25"),
        gun(1, "Gun 2", "056 124", "30"),
    ];
    let wind = WindDraft {
        speed_m_s: "4".into(),
        from_deg: "200".into(),
    };
    let drafts = MissionDrafts {
        burst_height: "500",
        ..drafts(&chosen, &target, &guns, &wind)
    };
    let mission = solve_mission(&catalog, drafts, |_, _| None).expect("solves");
    let engine = solve_fire_mission(&catalog, &mission.inputs).expect("the engine solves");
    assert_eq!(
        serde_json::to_string(&mission.solution).unwrap(),
        serde_json::to_string(&engine).unwrap()
    );
    assert_eq!(mission.target_grid, "064 129");
    assert_eq!(mission.solution.catalog_id, "mortar-page-test");
    assert_eq!(mission.solution.catalog_version, 2);
    assert!(
        mission.solution.fuze.is_some(),
        "a burst height sets the fuze"
    );
    let labels: Vec<&str> = mission
        .solution
        .guns
        .iter()
        .map(|g| g.label.as_str())
        .collect();
    assert_eq!(labels, ["Gun 1", "Gun 2"]);
}

#[test]
fn every_gun_is_solved_as_if_it_fired_alone() {
    let catalog = catalog();
    let chosen = selection("m252", "m821", ChargeChoice::Recommended);
    let target = manual("064 129", "40");
    let guns = [
        gun(0, "North", "064 139", "60"),
        gun(1, "West", "052 129", "10"),
        gun(2, "South east", "070 121", "35.5"),
    ];
    let calm = WindDraft::default();
    let battery = solve_mission(&catalog, drafts(&chosen, &target, &guns, &calm), |_, _| {
        None
    })
    .expect("solves");
    for (index, single) in guns.iter().enumerate() {
        let alone = solve_mission(
            &catalog,
            drafts(&chosen, &target, std::slice::from_ref(single), &calm),
            |_, _| None,
        )
        .expect("solves");
        assert_eq!(battery.solution.guns[index].label, single.label);
        assert_eq!(
            battery.solution.guns[index].charges,
            alone.solution.guns[0].charges
        );
        assert_eq!(
            battery.solution.guns[index].recommended_rings,
            alone.solution.guns[0].recommended_rings
        );
    }
}

#[test]
fn a_crosswind_is_aimed_into_and_calm_air_lays_on_the_target_line() {
    let catalog = catalog();
    let chosen = selection("m252", "m821", ChargeChoice::Recommended);
    let target = manual("064 139", "0");
    let guns = [gun(0, "Gun 1", "064 129", "0")];
    let laid_deflection = |wind: &WindDraft| {
        let mission = solve_mission(&catalog, drafts(&chosen, &target, &guns, wind), |_, _| None)
            .expect("solves");
        let gun = &mission.solution.guns[0];
        let rings = laid_rings(gun, mission.inputs.charge_rings).expect("a charge solves");
        gun.charges
            .iter()
            .find(|c| c.rings == rings)
            .and_then(|c| c.deflection_correction_mils)
            .expect("the laid charge solves")
    };
    assert_eq!(laid_deflection(&WindDraft::default()), 0.0);
    let from_west = WindDraft {
        speed_m_s: "8".into(),
        from_deg: "270".into(),
    };
    let deflection = laid_deflection(&from_west);
    assert!(
        deflection < 0.0,
        "a wind from the west is aimed into, left: {deflection}"
    );
}

#[test]
fn every_input_problem_is_reported_at_once() {
    let catalog = catalog();
    let chosen = selection("m252", "m853a1", ChargeChoice::Rings(0));
    let target = manual("064 12", "");
    let guns = [
        gun(0, "Gun 1", "055 125", "abc"),
        gun(1, "Gun 1", "055 126", "3"),
    ];
    let wind = WindDraft {
        speed_m_s: "-2".into(),
        from_deg: "90".into(),
    };
    let drafts = MissionDrafts {
        burst_height: "-10",
        ..drafts(&chosen, &target, &guns, &wind)
    };
    let problems = solve_mission(&catalog, drafts, |_, _| None).unwrap_err();
    assert_eq!(problems.len(), 6, "{problems:#?}");
    assert_eq!(problems[0], "M853A1 Illumination has no charge 0.");
    assert!(problems[1].starts_with("Target: Grid:"), "{}", problems[1]);
    assert!(
        problems[2].starts_with("Gun 1: Height \"abc\""),
        "{}",
        problems[2]
    );
    assert_eq!(problems[3], "Two guns are labelled \"Gun 1\".");
    assert!(
        problems[4].starts_with("Wind speed \"-2\""),
        "{}",
        problems[4]
    );
    assert!(
        problems[5].starts_with("Burst height \"-10\""),
        "{}",
        problems[5]
    );
}

#[test]
fn a_missing_selection_is_a_problem_and_an_engine_refusal_is_reported() {
    let catalog = catalog();
    let target = manual("064 129", "0");
    let guns = [gun(0, "Gun 1", "055 125", "0")];
    let calm = WindDraft::default();
    let empty = ArmamentSelection::default();
    assert_eq!(
        solve_mission(&catalog, drafts(&empty, &target, &guns, &calm), |_, _| None),
        Err(vec!["Pick a weapon and a shell.".to_string()])
    );
    let mismatched = selection("2b14", "m821", ChargeChoice::Recommended);
    let refused = solve_mission(
        &catalog,
        drafts(&mismatched, &target, &guns, &calm),
        |_, _| None,
    )
    .unwrap_err();
    assert_eq!(refused.len(), 1);
    assert!(
        refused[0].contains("does not fire shell `m821`"),
        "{}",
        refused[0]
    );
}

#[test]
fn a_burst_height_for_a_shell_without_a_time_fuze_is_dropped_before_the_engine() {
    let catalog = catalog();
    let he = selection("m252", "m821", ChargeChoice::Recommended);
    let target = manual("064 129", "0");
    let guns = [gun(0, "Gun 1", "055 125", "0")];
    let calm = WindDraft::default();
    let drafts = MissionDrafts {
        burst_height: "600",
        ..drafts(&he, &target, &guns, &calm)
    };
    let mission = solve_mission(&catalog, drafts, |_, _| None).expect("solves");
    assert_eq!(mission.inputs.burst_height_m, None);
    assert_eq!(mission.solution.fuze, None);
    assert_eq!(mission.inputs.charge_rings, None);
}

fn solved_row(rings: u32) -> ChargeSolution {
    ChargeSolution {
        rings,
        elevation_deg: Some(60.0),
        elevation_mils: Some(1_066.666),
        time_of_flight_s: Some(21.44),
        apex_m: Some(512.4),
        aim_azimuth_deg: Some(91.5),
        aim_azimuth_mils: Some(1_626.7),
        deflection_correction_mils: Some(-3.21),
        range_correction_m: Some(12.34),
        refusal: None,
    }
}

#[test]
fn a_solved_charge_row_shows_mils_degrees_and_the_aim_corrections() {
    assert_eq!(
        charge_row_text(&solved_row(2), Some(2)),
        ChargeRowText {
            charge: "Charge 2".into(),
            laid: true,
            elevation: "1066.7 mils · 60.00°".into(),
            aim_azimuth: "1626.7 mils · 91.50°".into(),
            deflection: "L 3.2 mils".into(),
            range_correction: "+12.3 m".into(),
            time_of_flight: "21.4 s".into(),
            apex: "512 m".into(),
        }
    );
    assert!(!charge_row_text(&solved_row(2), Some(3)).laid);
    assert!(!charge_row_text(&solved_row(2), None).laid);
    assert_eq!(deflection_text(4.26), "R 4.3 mils");
    assert_eq!(deflection_text(0.01), "0.0 mils");
    assert_eq!(range_correction_text(-7.0), "-7.0 m");
    assert_eq!(range_correction_text(0.0), "0.0 m");
}

#[test]
fn a_refused_charge_row_shows_the_refusal_and_no_figures() {
    for (refusal, text) in [
        (SolutionRefusal::TooClose, "too close for this charge"),
        (SolutionRefusal::OutOfRange, "out of range"),
        (
            SolutionRefusal::Unreachable,
            "target above the flight's apex",
        ),
        (SolutionRefusal::DidNotConverge, "no stable solution found"),
        (
            SolutionRefusal::TimeToLiveExceeded,
            "shell expires before impact",
        ),
        (SolutionRefusal::InvalidInput, "invalid input"),
    ] {
        let refused = ChargeSolution::from_result(1, Err(refusal), MilsConvention::MILS_6400);
        let row = charge_row_text(&refused, Some(1));
        assert_eq!(row.elevation, text);
        assert!(
            row.laid,
            "a chosen charge stays marked even when it refuses"
        );
        for empty in [
            &row.aim_azimuth,
            &row.deflection,
            &row.range_correction,
            &row.time_of_flight,
            &row.apex,
        ] {
            assert!(empty.is_empty());
        }
    }
}

#[test]
fn the_laid_charge_is_the_pinned_one_else_the_recommendation() {
    let catalog = catalog();
    let chosen = selection("m252", "m821", ChargeChoice::Recommended);
    let target = manual("064 129", "0");
    let guns = [gun(0, "Gun 1", "055 125", "0")];
    let calm = WindDraft::default();
    let mission = solve_mission(&catalog, drafts(&chosen, &target, &guns, &calm), |_, _| {
        None
    })
    .expect("solves");
    let gun = &mission.solution.guns[0];
    assert_eq!(laid_rings(gun, None), gun.recommended_rings);
    assert_eq!(laid_rings(gun, Some(4)), Some(4));
    let lowest_solving = gun
        .charges
        .iter()
        .filter(|c| c.solves())
        .map(|c| c.rings)
        .min();
    assert!(lowest_solving.is_some(), "984 m is inside the M821's reach");
    assert_eq!(gun.recommended_rings, lowest_solving);
    let heading = gun_heading(gun);
    assert!(heading.starts_with("Gun 1 — 985 m · line "), "{heading}");
    assert!(heading.ends_with("· Δh +0.0 m"), "{heading}");
}
