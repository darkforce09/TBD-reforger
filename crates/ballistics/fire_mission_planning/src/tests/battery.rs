//! Tests of the battery solution: one gun equals the one-gun solution, guns are independent,
//! the aim is the wind-corrected one, each gun carries its recommended charge's dispersion, and
//! malformed batteries are refused with the gun's index.

use std::sync::LazyLock;

use super::*;
use ballistics_model::catalog::BallisticsCatalog;
use ballistics_model::ids::{ShellId, WeaponId};
use ballistics_model::wind::Wind;
use ballistics_solver::dispersion::charge_dispersion;
use ballistics_solver::{FireSolutionRequest, MapPosition, solve_fire_solution};

/// The catalog shell `m821-he`, borrowed by the requests the tests build.
static SHELL_M821_HE: LazyLock<ShellId> = LazyLock::new(|| ShellId::new("m821-he"));
/// The catalog shell `no-such-shell`, borrowed by the requests the tests build.
static SHELL_NO_SUCH_SHELL: LazyLock<ShellId> = LazyLock::new(|| ShellId::new("no-such-shell"));
/// The catalog launcher `m252`, borrowed by the requests the tests build.
static WEAPON_M252: LazyLock<WeaponId> = LazyLock::new(|| WeaponId::new("m252"));

/// The hand-written test catalog (`m252` fires `m821-he`, rings 0/1/2).
fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/fixtures/ballistics/minimal_catalog.json"
    )))
    .expect("the sample catalog decodes")
}

fn at(x_m: f64, y_m: f64, height_m: f64) -> MapPosition {
    MapPosition { x_m, y_m, height_m }
}

fn gun(label: &str, position: MapPosition) -> BatteryGun {
    BatteryGun {
        label: label.to_owned(),
        position,
    }
}

const TARGET: MapPosition = MapPosition {
    x_m: 2_600.0,
    y_m: 3_400.0,
    height_m: 140.0,
};

const CROSSWIND: Wind = Wind {
    speed_m_s: 6.0,
    from_deg: 250.0,
};

fn battery<'r>(guns: &'r [BatteryGun], wind: Wind) -> BatteryRequest<'r> {
    BatteryRequest {
        weapon_id: &WEAPON_M252,
        shell_id: &SHELL_M821_HE,
        guns,
        target: TARGET,
        wind,
    }
}

fn one_gun(position: MapPosition, wind: Wind) -> FireSolutionRequest<'static> {
    FireSolutionRequest {
        weapon_id: &WEAPON_M252,
        shell_id: &SHELL_M821_HE,
        gun: position,
        target: TARGET,
        wind,
    }
}

#[test]
fn a_battery_of_one_gun_equals_the_one_gun_solution() {
    let catalog = catalog();
    for wind in [Wind::CALM, CROSSWIND] {
        let position = at(2_000.0, 2_900.0, 120.0);
        let guns = [gun("gun-1", position)];
        let answers = solve_battery(&catalog, &battery(&guns, wind)).expect("the battery solves");
        let single = solve_fire_solution(&catalog, &one_gun(position, wind)).expect("one gun");
        assert_eq!(answers.len(), 1);
        let recommended = single
            .recommended_charge()
            .expect("the fixture target is in range");
        let dispersion = charge_dispersion(&catalog, &one_gun(position, wind), recommended)
            .expect("the recommended charge disperses")
            .dispersion;
        let expected = GunFireSolution::from_fire_solution(
            0,
            "gun-1".to_owned(),
            single.clone(),
            Some(dispersion),
        );
        assert_eq!(answers[0], expected);
        assert_eq!(answers[0].gun_index, 0);
        assert_eq!(answers[0].label, "gun-1");
        assert_eq!(answers[0].distance_m.to_bits(), single.distance_m.to_bits());
        assert_eq!(
            answers[0].azimuth_mils.to_bits(),
            single.azimuth_mils.to_bits()
        );
        assert_eq!(answers[0].charges, single.charges);
        assert_eq!(answers[0].recommended_rings, single.recommended_rings);
        assert!(
            single.recommended_rings.is_some(),
            "the fixture target is in range"
        );
        assert_eq!(
            answers[0].recommended_charge(),
            single.recommended_charge(),
            "the recommended row is the one-gun row"
        );
    }
}

#[test]
fn each_gun_is_solved_independently_of_the_others() {
    let catalog = catalog();
    let positions = [
        at(2_000.0, 2_900.0, 120.0),
        at(2_040.0, 2_870.0, 118.0),
        at(2_150.0, 3_050.0, 90.0),
    ];
    let guns: Vec<BatteryGun> = positions
        .iter()
        .enumerate()
        .map(|(index, position)| gun(&format!("gun-{}", index + 1), *position))
        .collect();
    let answers = solve_battery(&catalog, &battery(&guns, CROSSWIND)).expect("the battery solves");
    assert_eq!(answers.len(), 3);
    for (index, answer) in answers.iter().enumerate() {
        let alone = solve_battery(&catalog, &battery(&guns[index..=index], CROSSWIND))
            .expect("the gun alone solves");
        assert_eq!(answer.gun_index as usize, index);
        assert_eq!(
            GunFireSolution {
                gun_index: 0,
                ..answer.clone()
            },
            alone[0],
            "gun {index} in the battery answers as it does alone"
        );
    }

    let mut reversed = guns.clone();
    reversed.reverse();
    let reversed_answers =
        solve_battery(&catalog, &battery(&reversed, CROSSWIND)).expect("the battery solves");
    for (index, answer) in reversed_answers.iter().enumerate() {
        let original = &answers[guns.len() - 1 - index];
        assert_eq!(answer.gun_index as usize, index);
        assert_eq!(answer.label, original.label);
        assert_eq!(
            answer.charges, original.charges,
            "order does not change a gun's rows"
        );
    }

    let aims: Vec<f64> = answers
        .iter()
        .map(|answer| {
            answer
                .recommended_charge()
                .and_then(|charge| charge.aim_azimuth_mils)
                .expect("every gun has a recommended aim")
        })
        .collect();
    assert!(aims[0] != aims[1] && aims[1] != aims[2] && aims[0] != aims[2]);
    assert!(answers[0].distance_m != answers[2].distance_m);
}

#[test]
fn the_aim_to_lay_is_the_wind_corrected_azimuth_of_the_charge_row() {
    let catalog = catalog();
    let guns = [gun("gun-1", at(2_000.0, 2_900.0, 120.0))];
    let calm = solve_battery(&catalog, &battery(&guns, Wind::CALM)).expect("calm solves");
    let windy = solve_battery(&catalog, &battery(&guns, CROSSWIND)).expect("wind solves");
    assert_eq!(
        calm[0].azimuth_mils.to_bits(),
        windy[0].azimuth_mils.to_bits()
    );
    let calm_row = calm[0].recommended_charge().expect("calm row");
    let windy_row = windy[0].recommended_charge().expect("windy row");
    assert_eq!(calm_row.aim_azimuth_mils, Some(calm[0].azimuth_mils));
    assert_eq!(calm_row.deflection_correction_mils, Some(0.0));
    let deflection = windy_row
        .deflection_correction_mils
        .expect("a solved row has a deflection correction");
    assert!(
        deflection.abs() > 1.0,
        "a 6 m/s crosswind moves the aim (got {deflection} mils)"
    );
    let aim = windy_row.aim_azimuth_mils.expect("a solved row has an aim");
    let expected_aim = (windy[0].azimuth_mils + deflection).rem_euclid(6_400.0);
    assert!(
        (aim - expected_aim).abs() < 1e-6,
        "aim {aim} vs {expected_aim}"
    );
}

#[test]
fn a_battery_without_guns_is_refused() {
    assert_eq!(
        solve_battery(&catalog(), &battery(&[], Wind::CALM)),
        Err(BatteryError::NoGuns)
    );
}

#[test]
fn a_blank_label_is_refused_with_its_gun_index() {
    let guns = [
        gun("gun-1", at(2_000.0, 2_900.0, 120.0)),
        gun("  ", at(2_040.0, 2_870.0, 118.0)),
    ];
    assert_eq!(
        solve_battery(&catalog(), &battery(&guns, Wind::CALM)),
        Err(BatteryError::EmptyLabel { gun_index: 1 })
    );
}

#[test]
fn an_invalid_gun_fails_the_battery_with_its_index() {
    let guns = [
        gun("gun-1", at(2_000.0, 2_900.0, 120.0)),
        gun("gun-2", at(2_040.0, 2_870.0, 118.0)),
        gun("gun-3", at(f64::NAN, 2_870.0, 118.0)),
    ];
    match solve_battery(&catalog(), &battery(&guns, Wind::CALM)) {
        Err(BatteryError::Gun {
            gun_index: 2,
            source: FireSolutionError::InvalidInput { parameter, .. },
        }) => assert_eq!(parameter, "gun.x_m"),
        other => panic!("expected gun 2's invalid input, got {other:?}"),
    }
}

#[test]
fn an_unknown_shell_fails_at_the_first_gun() {
    let guns = [gun("gun-1", at(2_000.0, 2_900.0, 120.0))];
    let request = BatteryRequest {
        shell_id: &SHELL_NO_SUCH_SHELL,
        ..battery(&guns, Wind::CALM)
    };
    assert!(matches!(
        solve_battery(&catalog(), &request),
        Err(BatteryError::Gun {
            gun_index: 0,
            source: FireSolutionError::Lookup(_),
        })
    ));
}

#[test]
fn a_gun_answer_round_trips_through_its_wire_form() {
    let guns = [gun("gun-1", at(2_000.0, 2_900.0, 120.0))];
    let answers = solve_battery(&catalog(), &battery(&guns, CROSSWIND)).expect("solves");
    let wire = serde_json::to_value(&answers[0]).expect("serialises");
    for key in [
        "gun_index",
        "label",
        "distance_m",
        "height_difference_m",
        "azimuth_deg",
        "azimuth_mils",
        "mils_per_circle",
        "charges",
        "recommended_rings",
        "dispersion",
    ] {
        assert!(wire.get(key).is_some(), "the wire form carries `{key}`");
    }
    let back: GunFireSolution = serde_json::from_value(wire).expect("deserialises");
    assert_eq!(back, answers[0]);
}

#[test]
fn each_gun_carries_the_dispersion_of_its_own_recommended_charge() {
    let catalog = catalog();
    let near = at(2_300.0, 3_150.0, 130.0);
    let far = at(2_050.0, 2_950.0, 100.0);
    let guns = [gun("near", near), gun("far", far)];
    let answers = solve_battery(&catalog, &battery(&guns, CROSSWIND)).expect("the battery solves");
    for (answer, position) in answers.iter().zip([near, far]) {
        let dispersion = answer.dispersion.expect("a solved gun has a dispersion");
        assert!(!dispersion.verified_in_engine);
        let expected = charge_dispersion(
            &catalog,
            &one_gun(position, CROSSWIND),
            answer.recommended_charge().expect("recommended"),
        )
        .expect("disperses")
        .dispersion;
        assert_eq!(dispersion, expected, "gun {}", answer.label);
    }
    let near_spread = answers[0].dispersion.expect("near").ellipse_semi_major_m;
    let far_spread = answers[1].dispersion.expect("far").ellipse_semi_major_m;
    assert!(
        near_spread != far_spread,
        "each gun disperses at its own range"
    );
}

#[test]
fn a_gun_with_no_solving_charge_has_no_dispersion() {
    let catalog = catalog();
    let guns = [
        gun("in-range", at(2_000.0, 2_900.0, 120.0)),
        gun("out-of-range", at(-40_000.0, 3_400.0, 120.0)),
    ];
    let answers = solve_battery(&catalog, &battery(&guns, Wind::CALM)).expect("the battery solves");
    assert!(answers[0].dispersion.is_some());
    assert_eq!(answers[1].recommended_rings, None);
    assert_eq!(answers[1].dispersion, None);
    assert!(answers[1].charges.iter().all(|charge| !charge.solves()));
}

#[test]
fn invalid_catalog_dispersion_values_fail_the_battery_with_the_gun_index() {
    let mut catalog = catalog();
    for weapon in &mut catalog.weapons {
        weapon.dispersion_range_m = 0.0;
    }
    let guns = [gun("gun-1", at(2_000.0, 2_900.0, 120.0))];
    assert!(matches!(
        solve_battery(&catalog, &battery(&guns, Wind::CALM)),
        Err(BatteryError::Dispersion { gun_index: 0, .. })
    ));
}
