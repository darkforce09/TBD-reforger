//! The high-angle elevation and wind-corrected aim of one charge, and every charge of a shell
//! evaluated in turn.
//!
//! **Role:** solves the aim (azimuth and elevation) at which one charge's shell descends through
//! the target height on the target, refuses with a typed [`SolutionRefusal`] when it cannot, and
//! picks the recommended charge: the one with the fewest rings that solves.
//!
//! **Position:** `ballistics/solver`; [`crate::data::scenario::ballistics::solver::fire_solution`]
//! resolves the catalog and the geometry into a [`ChargeProblem`] per charge; this module flies
//! [`crate::data::scenario::ballistics::flight_model::fly_to_height`] inside the searches of
//! [`crate::data::scenario::ballistics::solver::elevation_search`], and
//! [`crate::data::scenario::ballistics::solver::wind_corrected_aim`] repeats the elevation solve
//! along a moving aim line when the wind blows.
//!
//! **Signals & state:** none; pure functions over plain values.
//!
//! **Invariants:**
//! - Range is the downrange distance along the aim azimuth at the descending crossing of the
//!   target height. In calm air the aim is the gun-to-target line and the wind-corrected aim
//!   loop is skipped, so the result is the geometric solution bit for bit.
//! - Refusal order is fixed: a flight at the top of the elevation bracket that fails (invalid
//!   input, target above its apex, no elevation landing within the lifetime), then
//!   [`SolutionRefusal::TooClose`] when that flight's range exceeds the distance, then
//!   [`SolutionRefusal::OutOfRange`] when the maximum range falls short, then
//!   [`SolutionRefusal::DidNotConverge`] from the root search or the aim loop. Along an aim
//!   line, a range bound (too close, out of range) carries the bracket-end flight nearest the
//!   distance, so the wind-corrected aim moves off a refused aim line by that flight's drift.
//! - The top of the bracket is the weapon's maximum elevation, or, when that flight outlasts
//!   the shell's lifetime, the highest elevation that lands in time (bisection, at most
//!   [`RootSearchLimits::ELEVATION`] iterations): time of flight grows with the elevation, so
//!   the late elevations form one upper band. A capped top whose range still exceeds the distance
//!   is [`SolutionRefusal::TimeToLiveExceeded`], since every nearer landing comes too late.
//! - Only the high-angle branch is solved: the root lies in `[max(θ*, el_min), top]`, where
//!   `θ*` is the maximum-range elevation from a
//!   [`crate::data::scenario::ballistics::solver::elevation_search::GOLDEN_SECTION_ITERATIONS`]
//!   golden-section search.
//! - Every charge of the shell gets one row, in catalog order; no input panics.
//!
//! @contract fire-mission.schema.json#/definitions/ChargeSolution
//! @contract fire-mission.schema.json#/definitions/SolutionRefusal

use serde::{Deserialize, Serialize};

use super::elevation_search::{
    GOLDEN_SECTION_ITERATIONS, RootSearchError, RootSearchLimits, find_bracketed_root,
    maximise_by_golden_section,
};
use super::wind_corrected_aim::solve_wind_corrected_aim;
use crate::data::scenario::ballistics::angular_units::{
    MilsConvention, normalise_azimuth_degrees, radians_to_degrees,
};
use crate::data::scenario::ballistics::flight_model::{
    FlightError, FlightOutcome, FlightParameters, Launch, PathRecording, fly_to_height,
};
use crate::data::scenario::ballistics::wind::Wind;

/// Why a charge does not solve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolutionRefusal {
    /// The range at the maximum elevation exceeds the distance.
    TooClose,
    /// The maximum range falls short of the distance.
    OutOfRange,
    /// The target is above the apex of the flight at the maximum elevation.
    Unreachable,
    /// The elevation root search or the wind-corrected aim loop ran out of iterations.
    DidNotConverge,
    /// No elevation that reaches the target lands within the shell's lifetime.
    TimeToLiveExceeded,
    /// A NaN, infinite or non-physical input.
    InvalidInput,
}

impl From<FlightError> for SolutionRefusal {
    fn from(error: FlightError) -> Self {
        match error {
            FlightError::TargetAboveApex { .. } => Self::Unreachable,
            FlightError::TimeToLiveExceeded { .. } => Self::TimeToLiveExceeded,
            FlightError::InvalidInput { .. }
            | FlightError::InvalidWind(_)
            | FlightError::TooManyIntegrationSteps { .. } => Self::InvalidInput,
        }
    }
}

/// One charge's row: the aim, elevation, time of flight and apex when it solves, the refusal
/// when it does not (the numeric fields are then `None`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeSolution {
    /// Propellant rings of the charge.
    pub rings: u32,
    /// Elevation in degrees above the horizontal.
    pub elevation_deg: Option<f64>,
    /// Elevation in the weapon's mils.
    pub elevation_mils: Option<f64>,
    /// Seconds from firing to the descending crossing of the target height.
    pub time_of_flight_s: Option<f64>,
    /// Apex height above the gun, metres.
    pub apex_m: Option<f64>,
    /// Azimuth to lay in degrees, `[0, 360)`: the geometric azimuth plus the wind's
    /// deflection correction.
    pub aim_azimuth_deg: Option<f64>,
    /// Azimuth to lay in the weapon's mils, `[0, mils_per_circle)`.
    pub aim_azimuth_mils: Option<f64>,
    /// Aim azimuth minus the geometric azimuth in the weapon's mils, half a turn either way;
    /// positive lays right (clockwise) of the target line. Zero in calm air.
    pub deflection_correction_mils: Option<f64>,
    /// Horizontal distance to the aim point minus the gun-to-target distance, metres. Zero in
    /// calm air; the head- or tailwind share of the drift is already inside the elevation.
    pub range_correction_m: Option<f64>,
    /// Why the charge does not solve; `None` when it does.
    pub refusal: Option<SolutionRefusal>,
}

impl ChargeSolution {
    /// The row of a charge of `rings` rings from its elevation search result, with angles in
    /// the weapon's `mils` convention.
    pub fn from_result(
        rings: u32,
        result: Result<ChargeElevation, SolutionRefusal>,
        mils: MilsConvention,
    ) -> Self {
        match result {
            Ok(solved) => Self {
                rings,
                elevation_deg: Some(radians_to_degrees(solved.elevation_rad)),
                elevation_mils: Some(mils.radians_to_mils(solved.elevation_rad)),
                time_of_flight_s: Some(solved.time_of_flight_s),
                apex_m: Some(solved.apex_height_m),
                aim_azimuth_deg: Some(normalise_azimuth_degrees(radians_to_degrees(
                    solved.aim_azimuth_rad,
                ))),
                aim_azimuth_mils: Some(mils.azimuth_radians_to_mils(solved.aim_azimuth_rad)),
                deflection_correction_mils: Some(
                    mils.radians_to_mils(solved.deflection_correction_rad),
                ),
                range_correction_m: Some(solved.range_correction_m),
                refusal: None,
            },
            Err(refusal) => Self {
                rings,
                elevation_deg: None,
                elevation_mils: None,
                time_of_flight_s: None,
                apex_m: None,
                aim_azimuth_deg: None,
                aim_azimuth_mils: None,
                deflection_correction_mils: None,
                range_correction_m: None,
                refusal: Some(refusal),
            },
        }
    }

    /// Whether the charge solves.
    pub fn solves(&self) -> bool {
        self.refusal.is_none()
    }
}

/// One charge fired from the gun at the target: everything the aim search needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeProblem {
    /// The shell's physical constants.
    pub flight_parameters: FlightParameters,
    /// Muzzle speed of this charge, metres per second.
    pub muzzle_speed_m_s: f64,
    /// Lowest and highest elevation the weapon lays, radians.
    pub elevation_limits_rad: (f64, f64),
    /// Gun-to-target azimuth clockwise from north, radians.
    pub azimuth_rad: f64,
    /// Horizontal gun-to-target distance, metres.
    pub distance_m: f64,
    /// Target height minus gun height, metres.
    pub height_difference_m: f64,
    /// The surface wind.
    pub wind: Wind,
}

/// A solved charge: its aim, its high-angle elevation and the flight they produce.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeElevation {
    /// Elevation above the horizontal, radians.
    pub elevation_rad: f64,
    /// Seconds from firing to the descending crossing of the target height.
    pub time_of_flight_s: f64,
    /// Apex height above the gun, metres.
    pub apex_height_m: f64,
    /// Downrange distance along the aim azimuth at the crossing, metres.
    pub downrange_m: f64,
    /// Distance right of the aim azimuth at the crossing, metres (left is negative).
    pub deflection_m: f64,
    /// Azimuth to lay, clockwise from north, radians in `[0, 2π)`.
    pub aim_azimuth_rad: f64,
    /// Aim azimuth minus the geometric azimuth, radians in `[-π, π]`; positive is clockwise.
    pub deflection_correction_rad: f64,
    /// Horizontal distance to the aim point minus the gun-to-target distance, metres.
    pub range_correction_m: f64,
}

/// The aim and high-angle elevation that land `problem`'s shell on its target: along the
/// gun-to-target line in calm air, corrected for the wind's drift otherwise.
///
/// # Errors
///
/// A [`SolutionRefusal`] in the order the module invariants fix.
pub fn solve_charge_elevation(problem: &ChargeProblem) -> Result<ChargeElevation, SolutionRefusal> {
    let (lowest_rad, highest_rad) = problem.elevation_limits_rad;
    let distance_m = problem.distance_m;
    if !valid_elevation_limits(lowest_rad, highest_rad)
        || !distance_m.is_finite()
        || distance_m < 0.0
        || !problem.azimuth_rad.is_finite()
    {
        return Err(SolutionRefusal::InvalidInput);
    }
    if problem.wind.speed_m_s == 0.0 {
        let (solved, _) = solve_elevation_along(problem, problem.azimuth_rad, distance_m)
            .map_err(|refused| refused.refusal)?;
        return Ok(solved);
    }
    solve_wind_corrected_aim(problem)
}

/// A refused elevation solve along one aim line.
///
/// A range bound ([`SolutionRefusal::TooClose`] or [`SolutionRefusal::OutOfRange`]) carries the
/// flight at the bracket end whose range comes nearest the distance: the flight at the top of
/// the bracket, or at the maximum-range elevation. Its wind drift lets
/// [`crate::data::scenario::ballistics::solver::wind_corrected_aim`] move the aim point instead
/// of refusing the charge. Every other refusal carries no flight.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RefusedElevation {
    /// Why the elevation solve along the aim line refuses.
    pub(super) refusal: SolutionRefusal,
    /// The bracket-end flight nearest the distance, for a range bound.
    pub(super) nearest_flight: Option<FlightOutcome>,
}

impl From<SolutionRefusal> for RefusedElevation {
    fn from(refusal: SolutionRefusal) -> Self {
        Self {
            refusal,
            nearest_flight: None,
        }
    }
}

/// The high-angle elevation whose flight along `azimuth_rad` descends through the target height
/// `distance_m` downrange, with that flight. The row's aim is `azimuth_rad` and its corrections
/// are zero; the wind-corrected aim loop fills them in.
pub(super) fn solve_elevation_along(
    problem: &ChargeProblem,
    azimuth_rad: f64,
    distance_m: f64,
) -> Result<(ChargeElevation, FlightOutcome), RefusedElevation> {
    let (lowest_rad, highest_rad) = problem.elevation_limits_rad;
    let fly = |elevation_rad: f64| -> Result<FlightOutcome, FlightError> {
        let launch = Launch {
            muzzle_speed_m_s: problem.muzzle_speed_m_s,
            elevation_rad,
            azimuth_rad,
        };
        fly_to_height(
            &problem.flight_parameters,
            &launch,
            &problem.wind,
            problem.height_difference_m,
            PathRecording::Discard,
        )
    };

    let (top_rad, at_top) = match fly(highest_rad) {
        Ok(flight) => (highest_rad, flight),
        Err(FlightError::TimeToLiveExceeded { .. }) => {
            highest_elevation_landing_in_time(&fly, lowest_rad, highest_rad)?
        }
        Err(error) => return Err(SolutionRefusal::from(error).into()),
    };
    if at_top.downrange_m > distance_m {
        return Err(if top_rad < highest_rad {
            SolutionRefusal::TimeToLiveExceeded.into()
        } else {
            RefusedElevation {
                refusal: SolutionRefusal::TooClose,
                nearest_flight: Some(at_top),
            }
        });
    }
    let golden = maximise_by_golden_section(
        |elevation_rad| fly(elevation_rad).map_or(f64::NEG_INFINITY, |flight| flight.downrange_m),
        lowest_rad,
        top_rad,
        GOLDEN_SECTION_ITERATIONS,
    );
    let (maximum_range_rad, maximum_range_m) = if golden.value >= at_top.downrange_m {
        (golden.argument, golden.value)
    } else {
        (top_rad, at_top.downrange_m)
    };
    if maximum_range_m < distance_m {
        let nearest_flight = if maximum_range_rad == top_rad {
            Some(at_top)
        } else {
            fly(maximum_range_rad).ok()
        };
        return Err(RefusedElevation {
            refusal: SolutionRefusal::OutOfRange,
            nearest_flight,
        });
    }

    let root_rad = find_bracketed_root(
        |elevation_rad| fly(elevation_rad).map(|flight| flight.downrange_m - distance_m),
        (
            maximum_range_rad.max(lowest_rad),
            maximum_range_m - distance_m,
        ),
        (top_rad, at_top.downrange_m - distance_m),
        RootSearchLimits::ELEVATION,
    )
    .map_err(|error| match error {
        RootSearchError::Evaluation(flight_error) => SolutionRefusal::from(flight_error),
        RootSearchError::NonFiniteValue { .. } => SolutionRefusal::InvalidInput,
        RootSearchError::NoSignChange { .. } | RootSearchError::DidNotConverge { .. } => {
            SolutionRefusal::DidNotConverge
        }
    })?;
    let flight = fly(root_rad).map_err(SolutionRefusal::from)?;
    let solved = ChargeElevation {
        elevation_rad: root_rad,
        time_of_flight_s: flight.time_of_flight_s,
        apex_height_m: flight.apex_height_m,
        downrange_m: flight.downrange_m,
        deflection_m: flight.deflection_m,
        aim_azimuth_rad: azimuth_rad,
        deflection_correction_rad: 0.0,
        range_correction_m: 0.0,
    };
    Ok((solved, flight))
}

/// The highest elevation in `[lowest_rad, highest_rad]` whose flight does not outlast the
/// shell's lifetime, with that flight, given that the flight at `highest_rad` does.
///
/// Bisects the boundary between the elevations that land in time and the late band above them
/// to [`RootSearchLimits::ELEVATION`]'s tolerance within its iteration cap.
fn highest_elevation_landing_in_time(
    fly: &impl Fn(f64) -> Result<FlightOutcome, FlightError>,
    lowest_rad: f64,
    highest_rad: f64,
) -> Result<(f64, FlightOutcome), SolutionRefusal> {
    let lands_late = |flight: &Result<FlightOutcome, FlightError>| -> bool {
        matches!(flight, Err(FlightError::TimeToLiveExceeded { .. }))
    };
    let mut in_time_flight = fly(lowest_rad);
    if lands_late(&in_time_flight) {
        return Err(SolutionRefusal::TimeToLiveExceeded);
    }
    let limits = RootSearchLimits::ELEVATION;
    let (mut in_time_rad, mut late_rad) = (lowest_rad, highest_rad);
    for _ in 0..limits.max_iterations {
        if late_rad - in_time_rad <= limits.tolerance {
            break;
        }
        let middle_rad = 0.5 * (in_time_rad + late_rad);
        let flight = fly(middle_rad);
        if lands_late(&flight) {
            late_rad = middle_rad;
        } else {
            in_time_rad = middle_rad;
            in_time_flight = flight;
        }
    }
    match in_time_flight {
        Ok(flight) => Ok((in_time_rad, flight)),
        // The in-time elevations all fall short of the target height: none lands in time.
        Err(FlightError::TargetAboveApex { .. }) => Err(SolutionRefusal::TimeToLiveExceeded),
        Err(error) => Err(SolutionRefusal::from(error)),
    }
}

/// The ring count of the recommended charge: the fewest rings among the rows that solve.
pub fn recommended_rings(charges: &[ChargeSolution]) -> Option<u32> {
    charges
        .iter()
        .filter(|charge| charge.solves())
        .map(|charge| charge.rings)
        .min()
}

/// Elevation limits are finite and ordered; their range against the weapon's quarter turn is
/// checked where the catalog degrees are read.
fn valid_elevation_limits(lowest_rad: f64, highest_rad: f64) -> bool {
    lowest_rad.is_finite() && highest_rad.is_finite() && lowest_rad < highest_rad
}
