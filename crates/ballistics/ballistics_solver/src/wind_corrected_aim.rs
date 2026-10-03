//! The aim that lets the wind carry one charge's shell onto the target.
//!
//! **Role:** a fixed-point iteration on the aim point. The elevation is solved along the line
//! from the gun to the aim point, the flight's impact is compared with the target, and the aim
//! point moves by the miss: aim = target minus the wind's displacement of the current solution.
//!
//! **Position:** the `ballistics_solver` crate;
//! [`crate::charge_selection::solve_charge_elevation`] calls
//! it for any wind that blows and hands it the validated [`ChargeProblem`]; each iteration runs
//! the calm-air elevation solve of
//! [`crate::charge_selection`] along the current aim line.
//!
//! **Signals & state:** none; pure functions over plain values.
//!
//! **Invariants:**
//! - Map frame metres, x east, y north; the aim azimuth is `atan2(east, north)` of the aim point
//!   through `libm`, folded into `[0, 2π)`.
//! - The first aim point is the target itself, so a target the geometric aim line reaches takes
//!   the same iterations as a loop without range-bound recovery.
//! - An aim line refused by a range bound ([`SolutionRefusal::TooClose`],
//!   [`SolutionRefusal::OutOfRange`]) does not end the search: the next aim point is the target
//!   minus the drift of the bracket-end flight nearest the distance (its impact off the aim
//!   line), because a crosswind target at an elevation limit is often reachable only from an
//!   aim point upwind of it. The charge takes that range bound when the drift-cancelling aim
//!   point stays within [`AIM_MISS_TOLERANCE_M`] of the refused one, or when the last iteration
//!   is refused. Every other refusal along an aim line is the charge's at once.
//! - At most [`MAX_AIM_ITERATIONS`] elevation solves; the loop stops at the first whose impact
//!   lies under [`AIM_MISS_TOLERANCE_M`] from the target horizontally, else
//!   [`SolutionRefusal::DidNotConverge`] (or the range bound of a refused last iteration).
//! - The head- or tailwind share of the drift is absorbed by the in-wind elevation root along
//!   the aim line, so the aim point moves mostly across the line and the loop contracts by
//!   roughly the drift over the distance per iteration.

use super::charge_selection::{
    ChargeElevation, ChargeProblem, RefusedElevation, SolutionRefusal, solve_elevation_along,
};
use ballistics_model::angular_units::normalise_azimuth_radians;
use ballistics_model::flight_model::FlightOutcome;

/// The most elevation solves one charge's aim search runs.
pub const MAX_AIM_ITERATIONS: usize = 20;

/// Horizontal distance between the impact and the target under which the aim has converged,
/// metres.
pub const AIM_MISS_TOLERANCE_M: f64 = 0.01;

/// The wind-corrected aim and elevation of `problem`'s charge.
///
/// # Errors
///
/// A refusal of an elevation solve along an aim line other than a range bound; a range bound
/// ([`SolutionRefusal::TooClose`], [`SolutionRefusal::OutOfRange`]) when no aim point off the
/// refused line is reachable; or [`SolutionRefusal::DidNotConverge`] when
/// [`MAX_AIM_ITERATIONS`] solves leave the impact at least [`AIM_MISS_TOLERANCE_M`] from the
/// target.
pub fn solve_wind_corrected_aim(
    problem: &ChargeProblem,
) -> Result<ChargeElevation, SolutionRefusal> {
    let target_m = [
        problem.distance_m * libm::sin(problem.azimuth_rad),
        problem.distance_m * libm::cos(problem.azimuth_rad),
    ];
    let mut aim_m = target_m;
    let mut last_refusal = SolutionRefusal::DidNotConverge;
    for _ in 0..MAX_AIM_ITERATIONS {
        let aim_azimuth_rad = normalise_azimuth_radians(libm::atan2(aim_m[0], aim_m[1]));
        let aim_range_m = libm::hypot(aim_m[0], aim_m[1]);
        aim_m = match solve_elevation_along(problem, aim_azimuth_rad, aim_range_m) {
            Ok((solved, flight)) => {
                let miss_m = [
                    target_m[0] - flight.impact_position_m[0],
                    target_m[1] - flight.impact_position_m[1],
                ];
                if libm::hypot(miss_m[0], miss_m[1]) < AIM_MISS_TOLERANCE_M {
                    return Ok(ChargeElevation {
                        deflection_correction_rad: libm::remainder(
                            aim_azimuth_rad - problem.azimuth_rad,
                            core::f64::consts::TAU,
                        ),
                        range_correction_m: aim_range_m - problem.distance_m,
                        ..solved
                    });
                }
                last_refusal = SolutionRefusal::DidNotConverge;
                [aim_m[0] + miss_m[0], aim_m[1] + miss_m[1]]
            }
            Err(RefusedElevation {
                refusal,
                nearest_flight: Some(nearest_flight),
            }) => {
                let next_aim_m = aim_cancelling_drift(target_m, aim_azimuth_rad, &nearest_flight);
                let aim_step_m = libm::hypot(next_aim_m[0] - aim_m[0], next_aim_m[1] - aim_m[1]);
                if aim_step_m < AIM_MISS_TOLERANCE_M {
                    return Err(refusal);
                }
                last_refusal = refusal;
                next_aim_m
            }
            Err(refused) => return Err(refused.refusal),
        };
    }
    Err(last_refusal)
}

/// The aim point that cancels the wind drift of `flight`, flown along `aim_azimuth_rad`: the
/// target minus the flight's impact offset from the point `downrange_m` along the aim line.
///
/// The offset is the drift across the aim line; the drift along it is already part of the
/// flight's downrange, which the elevation root along the next aim line absorbs.
fn aim_cancelling_drift(
    target_m: [f64; 2],
    aim_azimuth_rad: f64,
    flight: &FlightOutcome,
) -> [f64; 2] {
    let on_aim_line_m = [
        flight.downrange_m * libm::sin(aim_azimuth_rad),
        flight.downrange_m * libm::cos(aim_azimuth_rad),
    ];
    [
        target_m[0] - (flight.impact_position_m[0] - on_aim_line_m[0]),
        target_m[1] - (flight.impact_position_m[1] - on_aim_line_m[1]),
    ]
}

#[cfg(test)]
#[path = "tests/wind_corrected_aim.rs"]
mod tests;
