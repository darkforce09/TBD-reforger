//! Tests of the time fuze: agreement of the burst-point inverse with a forward flight, the fuze
//! window refusals, the burst-point refusal causes, the search for the fewest rings that fuze,
//! and malformed requests.

use super::*;
use ballistics_model::angular_units::degrees_to_radians;
use ballistics_model::catalog::BallisticsCatalog;
use ballistics_model::flight_model::{Launch, PathRecording, fly_to_height};
use ballistics_model::ids::{ShellId, WeaponId};
use ballistics_model::wind::Wind;
use std::sync::LazyLock;

/// The catalog shell `m821-he`, borrowed by the requests the tests build.
static SHELL_M821_HE: LazyLock<ShellId> = LazyLock::new(|| ShellId::new("m821-he"));
/// The catalog shell `m853-illumination`, borrowed by the requests the tests build.
static SHELL_M853_ILLUMINATION: LazyLock<ShellId> =
    LazyLock::new(|| ShellId::new("m853-illumination"));
/// The catalog launcher `m252`, borrowed by the requests the tests build.
static WEAPON_M252: LazyLock<WeaponId> = LazyLock::new(|| WeaponId::new("m252"));

const WINDOW: TimeFuze = TimeFuze {
    min_s: 4.0,
    max_s: 50.0,
    default_s: 20.0,
};

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

fn illumination_request(target: MapPosition, wind: Wind) -> FireSolutionRequest<'static> {
    FireSolutionRequest {
        weapon_id: &WEAPON_M252,
        shell_id: &SHELL_M853_ILLUMINATION,
        gun: at(0.0, 0.0, 10.0),
        target,
        wind,
    }
}

#[test]
fn fuze_inverse_agrees_with_a_forward_flight_to_the_burst_point() {
    let catalog = catalog();
    let firing_parameters = catalog
        .resolve_firing(
            &WeaponId::new("m252"),
            &ShellId::new("m853-illumination"),
            1,
        )
        .expect("the charge exists");
    let mut compared = 0;
    for (target, burst_height_m, wind) in [
        (at(240.0, 180.0, 25.0), 150.0, Wind::CALM),
        (at(-250.0, 380.0, 0.0), 100.0, Wind::CALM),
        (
            at(300.0, -150.0, 40.0),
            150.0,
            Wind {
                speed_m_s: 7.0,
                from_deg: 250.0,
            },
        ),
    ] {
        let request = illumination_request(target, wind);
        let solution =
            solve_time_fuze(&catalog, &request, 1, burst_height_m).expect("the fuze computes");
        let time_s = solution
            .setting
            .time_s
            .unwrap_or_else(|| panic!("{target:?}: fuze refused {:?}", solution.setting.refusal));
        let aim = solution.burst_aim;
        let flight = fly_to_height(
            &firing_parameters.flight_parameters,
            &Launch {
                muzzle_speed_m_s: firing_parameters.muzzle_speed_m_s,
                elevation_rad: degrees_to_radians(aim.elevation_deg.expect("solved")),
                azimuth_rad: degrees_to_radians(aim.aim_azimuth_deg.expect("solved")),
            },
            &wind,
            target.height_m - request.gun.height_m + burst_height_m,
            PathRecording::Discard,
        )
        .expect("the forward flight reaches the burst height");
        assert!(
            (flight.time_of_flight_s - time_s).abs() <= 0.01,
            "{target:?}: fuze {time_s} s against forward flight {} s",
            flight.time_of_flight_s
        );
        let miss_m = libm::hypot(
            flight.impact_position_m[0] - (target.x_m - request.gun.x_m),
            flight.impact_position_m[1] - (target.y_m - request.gun.y_m),
        );
        assert!(
            miss_m < 0.05,
            "{target:?}: the burst misses the target by {miss_m} m"
        );
        assert_eq!(solution.setting.burst_height_m, burst_height_m);
        assert_eq!(solution.setting.default_s, 20.0);
        compared += 1;
    }
    assert_eq!(compared, 3);
}

#[test]
fn fuze_times_outside_the_window_are_refused() {
    for (time_s, accepted) in [
        (3.99, false),
        (4.0, true),
        (20.0, true),
        (50.0, true),
        (50.01, false),
    ] {
        let setting = fuze_setting(&WINDOW, 120.0, Ok(time_s));
        if accepted {
            assert_eq!(setting.time_s, Some(time_s));
            assert_eq!(setting.refusal, None);
        } else {
            assert_eq!(setting.time_s, None, "{time_s} s is outside the window");
            assert_eq!(setting.refusal, Some(FuzeRefusal::OutsideFuzeWindow));
        }
        assert_eq!(
            (setting.min_s, setting.max_s, setting.default_s),
            (4.0, 50.0, 20.0)
        );
    }

    let mut catalog = catalog();
    let request = illumination_request(at(300.0, 300.0, 10.0), Wind::CALM);
    let reachable_s = solve_time_fuze(&catalog, &request, 1, 120.0)
        .expect("the fuze computes")
        .setting
        .time_s
        .expect("inside the sample window");
    let shell = catalog
        .shells
        .iter_mut()
        .find(|shell| shell.shell_id == "m853-illumination")
        .expect("the shell exists");
    shell.time_fuze = Some(TimeFuze {
        min_s: reachable_s + 1.0,
        max_s: reachable_s + 10.0,
        default_s: reachable_s + 5.0,
    });
    let too_early = solve_time_fuze(&catalog, &request, 1, 120.0).expect("the fuze computes");
    assert_eq!(
        too_early.setting.refusal,
        Some(FuzeRefusal::OutsideFuzeWindow)
    );
    assert_eq!(too_early.setting.time_s, None);
    assert_eq!(too_early.setting.default_s, reachable_s + 5.0);
    assert_eq!(too_early.burst_aim.time_of_flight_s, Some(reachable_s));
}

#[test]
fn burst_above_the_apex_is_refused_as_above_the_apex() {
    let catalog = catalog();
    let request = illumination_request(at(300.0, 300.0, 10.0), Wind::CALM);
    let solution = solve_time_fuze(&catalog, &request, 0, 5_000.0).expect("the fuze computes");
    assert_eq!(solution.setting.refusal, Some(FuzeRefusal::AboveApex));
    assert_eq!(solution.setting.time_s, None);
    assert_eq!(
        solution.burst_aim.refusal,
        Some(SolutionRefusal::Unreachable)
    );
}

#[test]
fn every_solver_refusal_names_its_own_burst_point_cause() {
    let causes = [
        (SolutionRefusal::Unreachable, FuzeRefusal::AboveApex),
        (SolutionRefusal::OutOfRange, FuzeRefusal::BeyondRange),
        (SolutionRefusal::TooClose, FuzeRefusal::InsideMinimumRange),
        (SolutionRefusal::DidNotConverge, FuzeRefusal::DidNotConverge),
        (
            SolutionRefusal::TimeToLiveExceeded,
            FuzeRefusal::TimeToLiveExceeded,
        ),
        (SolutionRefusal::InvalidInput, FuzeRefusal::InvalidInput),
    ];
    for (solver, fuze) in causes {
        let setting = fuze_setting(&WINDOW, 100.0, Err(solver));
        assert_eq!(setting.refusal, Some(fuze), "{solver:?}");
        assert_eq!(setting.time_s, None);
        assert!(!fuze.reaches_burst_point(), "{fuze:?}");
    }
    assert!(FuzeRefusal::OutsideFuzeWindow.reaches_burst_point());
    let wire: Vec<_> = causes
        .iter()
        .map(|(_, fuze)| serde_json::to_value(fuze).expect("serialises"))
        .collect();
    assert_eq!(
        wire,
        [
            "above_apex",
            "beyond_range",
            "inside_minimum_range",
            "did_not_converge",
            "time_to_live_exceeded",
            "invalid_input",
        ]
    );

    let catalog = catalog();
    for (target, cause) in [
        (at(20_000.0, 20_000.0, 10.0), FuzeRefusal::BeyondRange),
        (at(1.0, 1.0, 10.0), FuzeRefusal::InsideMinimumRange),
    ] {
        let request = illumination_request(target, Wind::CALM);
        let searched = solve_time_fuze_over_charges(&catalog, &request, 50.0).expect("computes");
        assert_eq!(searched.setting.refusal, Some(cause), "{target:?}");
        assert_eq!(searched.burst_aim.rings, 0);
    }
}

#[test]
fn malformed_fuze_requests_are_typed_errors() {
    let catalog = catalog();
    let request = illumination_request(at(300.0, 300.0, 10.0), Wind::CALM);
    for burst_height_m in [f64::NAN, f64::INFINITY] {
        assert!(matches!(
            solve_time_fuze(&catalog, &request, 1, burst_height_m),
            Err(FuzeError::InvalidInput {
                parameter: "burst_height_m",
                ..
            })
        ));
    }
    assert!(matches!(
        solve_time_fuze(&catalog, &request, 9, 100.0),
        Err(FuzeError::Lookup(CatalogLookupError::UnknownRing {
            rings: 9,
            ..
        }))
    ));
    let high_explosive = FireSolutionRequest {
        shell_id: &SHELL_M821_HE,
        ..request
    };
    assert_eq!(
        solve_time_fuze(&catalog, &high_explosive, 1, 100.0),
        Err(FuzeError::ShellHasNoTimeFuze {
            shell_id: ShellId::new("m821-he"),
        })
    );
    let unplaced_gun = FireSolutionRequest {
        gun: at(f64::NAN, 0.0, 0.0),
        ..request
    };
    assert!(matches!(
        solve_time_fuze(&catalog, &unplaced_gun, 1, 100.0),
        Err(FuzeError::FireSolution(
            FireSolutionError::InvalidInput { .. }
        ))
    ));

    let mut malformed = catalog.clone();
    let shell = malformed
        .shells
        .iter_mut()
        .find(|shell| shell.shell_id == "m853-illumination")
        .expect("the shell exists");
    shell.time_fuze = Some(TimeFuze {
        min_s: 30.0,
        max_s: 10.0,
        default_s: 20.0,
    });
    assert!(matches!(
        solve_time_fuze(&malformed, &request, 1, 100.0),
        Err(FuzeError::InvalidInput {
            parameter: "max_s",
            ..
        })
    ));
}

#[test]
fn fuze_setting_serialises_the_contract_fields() {
    let refused = fuze_setting(&WINDOW, 150.0, Ok(60.0));
    let value = serde_json::to_value(refused).expect("serialises");
    assert_eq!(value["refusal"], "outside_fuze_window");
    assert_eq!(value["time_s"], serde_json::Value::Null);
    assert_eq!(value["burst_height_m"], 150.0);
    let accepted = serde_json::to_value(fuze_setting(&WINDOW, 150.0, Ok(21.5))).expect("ok");
    assert_eq!(accepted["refusal"], serde_json::Value::Null);
    assert_eq!(accepted["time_s"], 21.5);
    assert_eq!(accepted.as_object().expect("an object").len(), 6);
}

#[test]
fn the_charge_search_fuzes_at_the_fewest_rings_with_a_settable_time() {
    let catalog = catalog();
    let request = illumination_request(at(300.0, 300.0, 10.0), Wind::CALM);
    let searched = solve_time_fuze_over_charges(&catalog, &request, 120.0).expect("computes");
    let rings = searched.burst_aim.rings;
    assert!(searched.setting.time_s.is_some(), "{searched:?}");
    assert_eq!(
        searched,
        solve_time_fuze(&catalog, &request, rings, 120.0).expect("computes")
    );
    for lower in 0..rings {
        let fixed = solve_time_fuze(&catalog, &request, lower, 120.0).expect("computes");
        assert_eq!(fixed.setting.time_s, None, "charge {lower} would fuze");
    }

    let above_every_apex = solve_time_fuze_over_charges(&catalog, &request, 5_000.0).expect("ok");
    assert_eq!(
        above_every_apex.setting.refusal,
        Some(FuzeRefusal::AboveApex)
    );
    assert_eq!(above_every_apex.burst_aim.rings, 0);
    assert_eq!(
        above_every_apex.burst_aim.refusal,
        Some(SolutionRefusal::Unreachable)
    );

    let high_explosive = FireSolutionRequest {
        shell_id: &SHELL_M821_HE,
        ..request
    };
    assert_eq!(
        solve_time_fuze_over_charges(&catalog, &high_explosive, 100.0),
        Err(FuzeError::ShellHasNoTimeFuze {
            shell_id: ShellId::new("m821-he"),
        })
    );
    assert!(matches!(
        solve_time_fuze_over_charges(&catalog, &request, f64::NAN),
        Err(FuzeError::InvalidInput {
            parameter: "burst_height_m",
            ..
        })
    ));
    let mut chargeless = catalog.clone();
    chargeless
        .shells
        .iter_mut()
        .find(|shell| shell.shell_id == "m853-illumination")
        .expect("the shell exists")
        .charges
        .clear();
    assert_eq!(
        solve_time_fuze_over_charges(&chargeless, &request, 120.0),
        Err(FuzeError::ShellHasNoCharges {
            shell_id: ShellId::new("m853-illumination"),
        })
    );
}
