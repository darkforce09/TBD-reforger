//! The flight model of one shell at one muzzle speed coefficient, and the elevation criterion.
//!
//! **Role:** flies a catalog shell at one charge coefficient, answers the model's range window
//! over an elevation ± 1 mil (6400 convention), and judges one (elevation, range, time of flight)
//! case the way native rows, forward-angle samples and wind-table rows are judged.
//!
//! **Position:** `ballistics/calibration`; [`super::native_tables`], [`super::wind_tables`] and
//! [`super::oracle_samples`] build a [`ChargeFlights`] per shell and call
//! [`judge_elevation_case`]; it flies through [`crate::data::scenario::ballistics::flight_model`].
//!
//! **Signals & state:** [`ChargeFlights`] memoises one [`ChargeFlight`] per coefficient during
//! one evaluation; nothing outlives it.
//!
//! **Invariants:**
//! - Muzzle speed is the shell's initial speed times the coefficient: the game's tables and the
//!   oracle belong to the shell, not to a launcher, so no weapon coefficient enters.
//! - Flights start at the muzzle, azimuth north unless a case names another, and end at the
//!   descending crossing of height 0 unless a case names another target height.
//! - The range over `[θ - 1 mil, θ + 1 mil]` is unimodal in θ (one maximum at the model's
//!   maximum-range elevation, found once per coefficient by golden section over `[0, π/2]`), so
//!   its minimum is at an end and its maximum at an end or at that elevation.
//! - A row stating its elevation rounded is flown at the elevation the model's calm range pins
//!   inside the row's ± 1 mil window ([`ChargeFlight::calm_elevation_for_range`]), found by the
//!   solver's bracketed root search on the side of the maximum-range elevation holding the row.
//! - An elevation case passes iff its range lies in that window and, when it carries a time of
//!   flight, the model's time at θ is within [`TIME_OF_FLIGHT_TOLERANCE_S`].

use core::f64::consts::{FRAC_PI_2, TAU};

use super::report::{CalibrationReport, FailureKind};
use crate::data::scenario::ballistics::catalog::{Shell, flight_parameters};
use crate::data::scenario::ballistics::flight_model::{
    FlightError, FlightOutcome, FlightParameters, Launch, PathRecording, fly_to_height,
};
use crate::data::scenario::ballistics::solver::elevation_search::{
    GOLDEN_SECTION_ITERATIONS, RootSearchLimits, find_bracketed_root, maximise_by_golden_section,
};
use crate::data::scenario::ballistics::wind::Wind;

/// One mil of the 6400 convention in radians: the calibration's angular tolerance.
pub const ONE_MIL_6400_RAD: f64 = TAU / 6400.0;

/// Largest accepted time-of-flight difference in seconds.
pub const TIME_OF_FLIGHT_TOLERANCE_S: f64 = 0.1;

/// Coefficients closer than this name the same charge.
const COEFFICIENT_MATCH_TOLERANCE: f64 = 1e-9;

/// One shell fired at one muzzle speed coefficient.
#[derive(Debug, Clone)]
pub struct ChargeFlight {
    parameters: FlightParameters,
    muzzle_speed_m_s: f64,
    max_range_elevation_rad: f64,
}

/// The lowest and highest model range over an elevation window, metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeWindow {
    /// Smallest downrange distance in the window.
    pub lowest_m: f64,
    /// Largest downrange distance in the window.
    pub highest_m: f64,
}

impl RangeWindow {
    /// Whether `range_m` lies in the window, ends included.
    pub fn contains(&self, range_m: f64) -> bool {
        range_m >= self.lowest_m && range_m <= self.highest_m
    }
}

impl ChargeFlight {
    /// The flight of `shell` under `gravity_m_s2` at `init_speed_coef`, with its maximum-range
    /// elevation located.
    pub fn new(gravity_m_s2: f64, shell: &Shell, init_speed_coef: f64) -> Self {
        let mut flight = Self {
            parameters: flight_parameters(gravity_m_s2, shell),
            muzzle_speed_m_s: shell.init_speed_m_s * init_speed_coef,
            max_range_elevation_rad: FRAC_PI_2,
        };
        let maximum = maximise_by_golden_section(
            |elevation| {
                flight
                    .fly_calm(elevation)
                    .map_or(f64::NEG_INFINITY, |outcome| outcome.downrange_m)
            },
            0.0,
            FRAC_PI_2,
            GOLDEN_SECTION_ITERATIONS,
        );
        flight.max_range_elevation_rad = maximum.argument;
        flight
    }

    /// Flies at `elevation_rad` and `azimuth_rad` through `wind` to the descending crossing of
    /// `target_height_m`.
    ///
    /// # Errors
    ///
    /// The flight model's [`FlightError`].
    pub fn fly(
        &self,
        elevation_rad: f64,
        azimuth_rad: f64,
        wind: &Wind,
        target_height_m: f64,
    ) -> Result<FlightOutcome, FlightError> {
        let launch = Launch {
            muzzle_speed_m_s: self.muzzle_speed_m_s,
            elevation_rad,
            azimuth_rad,
        };
        fly_to_height(
            &self.parameters,
            &launch,
            wind,
            target_height_m,
            PathRecording::Discard,
        )
    }

    /// Flies at `elevation_rad` towards north without wind to the crossing of the muzzle height.
    ///
    /// # Errors
    ///
    /// The flight model's [`FlightError`].
    pub fn fly_calm(&self, elevation_rad: f64) -> Result<FlightOutcome, FlightError> {
        self.fly(elevation_rad, 0.0, &Wind::CALM, 0.0)
    }

    /// The elevation within `[elevation_rad - 1 mil, elevation_rad + 1 mil]` at which the
    /// model's calm range equals `range_m`, searched on the side of the maximum-range elevation
    /// that holds `elevation_rad`; `None` when that side of the window holds no such elevation.
    ///
    /// A table that states its elevations rounded (the wind tables carry radians to three
    /// decimals, up to half a mil off) still states each row's calm range to the millimetre,
    /// which pins the row's elevation to the model's precision.
    pub fn calm_elevation_for_range(&self, elevation_rad: f64, range_m: f64) -> Option<f64> {
        let (mut low, mut high) = (
            elevation_rad - ONE_MIL_6400_RAD,
            elevation_rad + ONE_MIL_6400_RAD,
        );
        let split = self.max_range_elevation_rad;
        if split > low && split < high {
            if elevation_rad >= split {
                low = split;
            } else {
                high = split;
            }
        }
        let miss = |elevation: f64| {
            self.fly_calm(elevation)
                .map(|outcome| outcome.downrange_m - range_m)
        };
        let (low_miss, high_miss) = (miss(low).ok()?, miss(high).ok()?);
        find_bracketed_root(
            miss,
            (low, low_miss),
            (high, high_miss),
            RootSearchLimits::ELEVATION,
        )
        .ok()
    }

    /// The model's calm range window over `[elevation_rad - 1 mil, elevation_rad + 1 mil]`.
    ///
    /// # Errors
    ///
    /// The flight model's [`FlightError`] for a window end or the maximum inside it.
    pub fn range_window(&self, elevation_rad: f64) -> Result<RangeWindow, FlightError> {
        let low = elevation_rad - ONE_MIL_6400_RAD;
        let high = elevation_rad + ONE_MIL_6400_RAD;
        let low_range = self.fly_calm(low)?.downrange_m;
        let high_range = self.fly_calm(high)?.downrange_m;
        let mut highest = low_range.max(high_range);
        if self.max_range_elevation_rad > low && self.max_range_elevation_rad < high {
            highest = highest.max(self.fly_calm(self.max_range_elevation_rad)?.downrange_m);
        }
        Ok(RangeWindow {
            lowest_m: low_range.min(high_range),
            highest_m: highest,
        })
    }
}

/// Every [`ChargeFlight`] of one shell, built on first use.
#[derive(Debug)]
pub struct ChargeFlights<'catalog> {
    gravity_m_s2: f64,
    shell: &'catalog Shell,
    flights: Vec<(f64, ChargeFlight)>,
}

impl<'catalog> ChargeFlights<'catalog> {
    /// No flight built yet for `shell` under `gravity_m_s2`.
    pub fn new(gravity_m_s2: f64, shell: &'catalog Shell) -> Self {
        Self {
            gravity_m_s2,
            shell,
            flights: Vec::new(),
        }
    }

    /// The flight at `init_speed_coef`, built on first request.
    pub fn at(&mut self, init_speed_coef: f64) -> &ChargeFlight {
        let found = self
            .flights
            .iter()
            .position(|(coefficient, _)| same_coefficient(*coefficient, init_speed_coef));
        let index = found.unwrap_or_else(|| {
            let flight = ChargeFlight::new(self.gravity_m_s2, self.shell, init_speed_coef);
            self.flights.push((init_speed_coef, flight));
            self.flights.len() - 1
        });
        &self.flights[index].1
    }
}

/// Whether two coefficients name the same charge.
pub fn same_coefficient(left: f64, right: f64) -> bool {
    (left - right).abs() <= COEFFICIENT_MATCH_TOLERANCE
}

/// What an elevation case claims and which failure classes it reports.
#[derive(Debug, Clone, Copy)]
pub struct ElevationCase {
    /// Elevation the case was fired at, radians.
    pub elevation_rad: f64,
    /// Range the game reports at that elevation, metres.
    pub range_m: f64,
    /// Time of flight the game reports, seconds, when the case carries one.
    pub time_of_flight_s: Option<f64>,
    /// Failure class of a range outside the window.
    pub range_failure: FailureKind,
    /// Failure class of a time of flight beyond tolerance.
    pub time_failure: FailureKind,
}

/// Judges one elevation case and records its failures under `case_id`.
pub fn judge_elevation_case(
    report: &mut CalibrationReport,
    flight: &ChargeFlight,
    case_id: &str,
    case: &ElevationCase,
) {
    report.cases += 1;
    let window = match flight.range_window(case.elevation_rad) {
        Ok(window) => window,
        Err(error) => return refuse(report, case_id, &error),
    };
    if !window.contains(case.range_m) {
        report.fail(
            case.range_failure,
            case_id.to_owned(),
            format!(
                "range {} m lies outside the model's {:.3}..{:.3} m over elevation {:.6} rad ± 1 mil",
                case.range_m, window.lowest_m, window.highest_m, case.elevation_rad
            ),
        );
    }
    let Some(expected_time_s) = case.time_of_flight_s else {
        return;
    };
    match flight.fly_calm(case.elevation_rad) {
        Ok(outcome)
            if (outcome.time_of_flight_s - expected_time_s).abs()
                <= TIME_OF_FLIGHT_TOLERANCE_S => {}
        Ok(outcome) => report.fail(
            case.time_failure,
            case_id.to_owned(),
            format!(
                "time of flight {expected_time_s} s differs from the model's {:.4} s by more than {TIME_OF_FLIGHT_TOLERANCE_S} s",
                outcome.time_of_flight_s
            ),
        ),
        Err(error) => refuse(report, case_id, &error),
    }
}

/// Records a flight the model refused.
pub fn refuse(report: &mut CalibrationReport, case_id: &str, error: &FlightError) {
    report.fail(
        FailureKind::ModelRefused,
        case_id.to_owned(),
        format!("the flight model refused the case: {error}"),
    );
}
