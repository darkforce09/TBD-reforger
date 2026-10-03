//! Tests of the firing solver's wind-corrected aim: every solving charge's aim, flown forward
//! through the flight model with the request's wind, lands within 0.05 m of the target under
//! cross, head, tail and quartering winds with height differences, and calm air keeps the
//! geometric aim bit for bit.

use super::*;

/// Horizontal distance, metres, between the target and the impact of `rings` fired at the row's
/// aim azimuth and elevation, flown forward through the flight model with the request's wind.
fn forward_miss_m(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    solution: &FireSolution,
    rings: u32,
    aim_azimuth_deg: f64,
) -> f64 {
    let resolved = catalog
        .resolve_firing(request.weapon_id, request.shell_id, rings)
        .expect("the ring exists");
    let charge = row(solution, rings);
    let launch = Launch {
        muzzle_speed_m_s: resolved.muzzle_speed_m_s,
        elevation_rad: degrees_to_radians(charge.elevation_deg.expect("solved")),
        azimuth_rad: degrees_to_radians(aim_azimuth_deg),
    };
    let flight = fly_to_height(
        &resolved.flight_parameters,
        &launch,
        &request.wind,
        request.target.height_m - request.gun.height_m,
        PathRecording::Discard,
    )
    .expect("the solved aim flies");
    libm::hypot(
        request.gun.x_m + flight.impact_position_m[0] - request.target.x_m,
        request.gun.y_m + flight.impact_position_m[1] - request.target.y_m,
    )
}

/// The rings of every charge that solves, asserting that at least one does.
fn solved_rings(solution: &FireSolution) -> Vec<u32> {
    let rings: Vec<u32> = solution
        .charges
        .iter()
        .filter(|charge| charge.solves())
        .map(|charge| charge.rings)
        .collect();
    assert!(
        !rings.is_empty(),
        "no charge solves: {:?}",
        refusals(solution)
    );
    rings
}

/// Solves `request` with drag and asserts every solving charge's aim lands within 0.05 m of the
/// target when flown forward; answers the solution.
fn assert_aim_lands_on_target(request: &FireSolutionRequest<'_>) -> FireSolution {
    let catalog = drag_catalog();
    let solution = solve_fire_solution(&catalog, request).expect("the request is valid");
    for rings in solved_rings(&solution) {
        let aim_deg = row(&solution, rings).aim_azimuth_deg.expect("solved");
        let miss_m = forward_miss_m(&catalog, request, &solution, rings, aim_deg);
        assert!(miss_m < 0.05, "ring {rings}: the aim misses by {miss_m} m");
    }
    solution
}

#[test]
fn a_crosswind_from_the_left_aims_left_of_the_target_and_lands_on_it() {
    let request = FireSolutionRequest {
        wind: Wind {
            speed_m_s: 8.0,
            from_deg: 270.0,
        },
        ..m252_request(at(0.0, 0.0, 0.0), at(0.0, 800.0, 0.0))
    };
    let solution = assert_aim_lands_on_target(&request);
    let catalog = drag_catalog();
    for rings in solved_rings(&solution) {
        let charge = row(&solution, rings);
        let correction_mils = charge.deflection_correction_mils.expect("solved");
        let aim_deg = charge.aim_azimuth_deg.expect("solved");
        assert!(
            correction_mils < -1.0,
            "ring {rings}: {correction_mils} mils"
        );
        assert!(
            aim_deg > 270.0 && aim_deg < 360.0,
            "ring {rings}: {aim_deg}°"
        );
        let aim_mils = charge.aim_azimuth_mils.expect("solved");
        assert!(
            (aim_mils - (6400.0 + correction_mils)).abs() < 1e-6,
            "{aim_mils}"
        );
        // The geometric bearing misses: the correction is what lands the shell.
        let geometric_miss_m =
            forward_miss_m(&catalog, &request, &solution, rings, solution.azimuth_deg);
        assert!(geometric_miss_m > 1.0, "ring {rings}: {geometric_miss_m} m");
    }
}

#[test]
fn head_and_tail_winds_land_on_the_target_along_the_target_line() {
    let calm = m252_request(at(0.0, 0.0, 0.0), at(0.0, 800.0, 0.0));
    let calm_solution = assert_aim_lands_on_target(&calm);
    for (from_deg, lower_than_calm) in [(0.0, true), (180.0, false)] {
        let request = FireSolutionRequest {
            wind: Wind {
                speed_m_s: 8.0,
                from_deg,
            },
            ..calm
        };
        let solution = assert_aim_lands_on_target(&request);
        for rings in solved_rings(&solution) {
            let charge = row(&solution, rings);
            let correction_mils = charge.deflection_correction_mils.expect("solved");
            assert!(
                correction_mils.abs() < 0.01,
                "from {from_deg}°: {correction_mils}"
            );
            // On the high-angle branch a headwind's shorter flight needs a flatter tube, a
            // tailwind's longer one a steeper tube.
            let elevation_deg = charge.elevation_deg.expect("solved");
            let calm_deg = row(&calm_solution, rings).elevation_deg.expect("solved");
            assert_eq!(
                elevation_deg < calm_deg,
                lower_than_calm,
                "from {from_deg}°"
            );
        }
    }
}

#[test]
fn a_quartering_wind_with_a_height_difference_lands_on_the_target() {
    for (gun, target) in [
        (at(100.0, 100.0, 40.0), at(700.0, 500.0, 15.0)),
        (at(-300.0, 250.0, 5.0), at(250.0, -350.0, 45.0)),
    ] {
        let request = FireSolutionRequest {
            wind: Wind {
                speed_m_s: 10.0,
                from_deg: 225.0,
            },
            ..m252_request(gun, target)
        };
        let solution = assert_aim_lands_on_target(&request);
        assert_ne!(solution.height_difference_m, 0.0);
        for rings in solved_rings(&solution) {
            let charge = row(&solution, rings);
            let correction_mils = charge.deflection_correction_mils.expect("solved");
            let range_correction_m = charge.range_correction_m.expect("solved");
            assert!(
                correction_mils.abs() > 1.0,
                "ring {rings}: {correction_mils}"
            );
            assert!(range_correction_m.is_finite(), "ring {rings}");
        }
    }
}

#[test]
fn calm_air_keeps_the_geometric_aim_bit_for_bit() {
    let catalog = drag_catalog();
    let calm = m252_request(at(100.0, 100.0, 40.0), at(700.0, 500.0, 15.0));
    let solution = solve_fire_solution(&catalog, &calm).expect("the request is valid");
    let still = FireSolutionRequest {
        wind: Wind {
            speed_m_s: 0.0,
            from_deg: 137.0,
        },
        ..calm
    };
    assert_eq!(
        solve_fire_solution(&catalog, &still).expect("the request is valid"),
        solution
    );
    for rings in solved_rings(&solution) {
        let charge = row(&solution, rings);
        let aim_deg = charge.aim_azimuth_deg.expect("solved");
        let aim_mils = charge.aim_azimuth_mils.expect("solved");
        assert_eq!(aim_deg.to_bits(), solution.azimuth_deg.to_bits());
        assert_eq!(aim_mils.to_bits(), solution.azimuth_mils.to_bits());
        assert_eq!(charge.deflection_correction_mils, Some(0.0));
        assert_eq!(charge.range_correction_m, Some(0.0));
    }
}
