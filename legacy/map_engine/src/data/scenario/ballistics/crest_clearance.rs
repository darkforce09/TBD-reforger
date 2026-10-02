//! Crest clearance: how far a solved trajectory passes above the terrain along the line of fire.
//!
//! **Role:** compares a recorded shell flight with a terrain profile sampled along the
//! gun-to-target line and reports the smallest clearance, where it occurs, and the first sample
//! the trajectory passes below.
//!
//! **Position:** `data/scenario/ballistics`; the caller samples the terrain (the DEM on the
//! page, a hand-built profile in tests) into a [`TerrainProfile`]; the flight comes from
//! [`crate::data::scenario::ballistics::flight_model::fly_to_height`] with
//! [`crate::data::scenario::ballistics::flight_model::PathRecording::KeepSamples`], either
//! directly through [`crest_clearance`] or re-flown from a solved charge through
//! [`crest_clearance_of_charge`]. The fire-mission solution carries the lead gun's result.
//!
//! **Signals & state:** none; pure functions over plain values.
//!
//! **Invariants:**
//! - Downrange is the horizontal distance along the gun-to-target line (azimuth clockwise from
//!   north): `east·sin(az) + north·cos(az)` through `libm`, so a wind-drifted flight is
//!   measured on the same axis as the profile.
//! - The trajectory height at a sample's downrange is the linear interpolation between the two
//!   recorded states that bracket it; when the flight passes over that downrange more than once
//!   the lowest height counts.
//! - Clearance is `trajectory height above the muzzle - (terrain height - gun height)`, metres;
//!   a sample blocks when its clearance is below zero. At the recorded impact the trajectory
//!   height equals the target height difference exactly, so terrain at the target height
//!   clears by exactly zero.
//! - Only samples within the flight's downrange span `[0, impact]` count; samples are taken in
//!   ascending downrange whatever their input order, so "first blocking" is the nearest one to
//!   the gun.
//! - Malformed inputs are a [`CrestClearanceError`]; nothing panics.
//!
//! @contract fire-mission.schema.json#/definitions/CrestClearance

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::data::scenario::ballistics::flight_model::{
    FlightError, FlightSample, Launch, PathRecording, fly_to_height,
};
use crate::data::scenario::ballistics::solver::{ChargeElevation, ChargeProblem};

/// One terrain height along the line of fire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainSample {
    /// Horizontal distance from the gun along the gun-to-target line, metres.
    pub downrange_m: f64,
    /// Terrain height, metres, in the same datum as the gun height.
    pub height_m: f64,
}

/// The terrain under the line of fire, as heights at downrange distances.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainProfile {
    /// Heights along the line of fire, in any order.
    pub samples: Vec<TerrainSample>,
}

/// How far the trajectory passes above the terrain profile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrestClearance {
    /// Smallest clearance over the counted samples, metres; negative when the terrain blocks.
    pub min_clearance_m: f64,
    /// Downrange of the smallest clearance, metres.
    pub min_clearance_downrange_m: f64,
    /// Downrange of the first sample the trajectory passes below; `None` when none does.
    pub first_blocking_downrange_m: Option<f64>,
}

impl CrestClearance {
    /// Whether any counted terrain sample rises above the trajectory.
    pub fn is_blocked(&self) -> bool {
        self.first_blocking_downrange_m.is_some()
    }
}

/// Why a crest clearance cannot be computed.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CrestClearanceError {
    /// A profile, gun or line value is NaN, infinite or out of its domain.
    #[error("crest clearance input `{parameter}` is invalid (got {value})")]
    InvalidInput {
        /// Name of the offending input.
        parameter: &'static str,
        /// The value that was refused.
        value: f64,
    },
    /// The recorded flight has fewer than two states.
    #[error("the recorded flight has fewer than two states")]
    FlightNotRecorded,
    /// No profile sample lies within the flight's downrange span.
    #[error("no terrain sample lies between the gun and the impact")]
    NoSampleUnderFlight,
    /// Re-flying the solved charge failed.
    #[error("the solved charge does not fly: {0}")]
    Flight(FlightError),
}

/// Clearance of the recorded `path` (muzzle-relative states in time order) over `profile`,
/// measured along the gun-to-target line at `line_azimuth_rad` from a gun at `gun_height_m`.
///
/// # Errors
///
/// [`CrestClearanceError::InvalidInput`] for a non-finite gun height, line azimuth, profile
/// value or recorded state, or a negative sample downrange;
/// [`CrestClearanceError::FlightNotRecorded`] for fewer than two states;
/// [`CrestClearanceError::NoSampleUnderFlight`] when every sample lies beyond the impact.
pub fn crest_clearance(
    path: &[FlightSample],
    line_azimuth_rad: f64,
    gun_height_m: f64,
    profile: &TerrainProfile,
) -> Result<CrestClearance, CrestClearanceError> {
    require("gun_height_m", gun_height_m, gun_height_m.is_finite())?;
    require(
        "line_azimuth_rad",
        line_azimuth_rad,
        line_azimuth_rad.is_finite(),
    )?;
    if path.len() < 2 {
        return Err(CrestClearanceError::FlightNotRecorded);
    }
    let (sin_azimuth, cos_azimuth) = (libm::sin(line_azimuth_rad), libm::cos(line_azimuth_rad));
    let mut flight: Vec<(f64, f64)> = Vec::with_capacity(path.len());
    for sample in path {
        let [east, north, up] = sample.position_m;
        let downrange = east * sin_azimuth + north * cos_azimuth;
        require("path.downrange_m", downrange, downrange.is_finite())?;
        require("path.height_m", up, up.is_finite())?;
        flight.push((downrange, up));
    }
    let impact_downrange_m = flight[flight.len() - 1].0;

    let mut samples = profile.samples.clone();
    for sample in &samples {
        require(
            "profile.downrange_m",
            sample.downrange_m,
            sample.downrange_m.is_finite() && sample.downrange_m >= 0.0,
        )?;
        require(
            "profile.height_m",
            sample.height_m,
            sample.height_m.is_finite(),
        )?;
    }
    samples.sort_by(|left, right| left.downrange_m.total_cmp(&right.downrange_m));

    let mut lowest: Option<(f64, f64)> = None;
    let mut first_blocking_downrange_m = None;
    for sample in samples
        .iter()
        .filter(|sample| sample.downrange_m <= impact_downrange_m)
    {
        let Some(height_above_muzzle_m) = lowest_height_at(&flight, sample.downrange_m) else {
            continue;
        };
        let clearance_m = height_above_muzzle_m - (sample.height_m - gun_height_m);
        if lowest.is_none_or(|(least_m, _)| clearance_m < least_m) {
            lowest = Some((clearance_m, sample.downrange_m));
        }
        if clearance_m < 0.0 && first_blocking_downrange_m.is_none() {
            first_blocking_downrange_m = Some(sample.downrange_m);
        }
    }
    let (min_clearance_m, min_clearance_downrange_m) =
        lowest.ok_or(CrestClearanceError::NoSampleUnderFlight)?;
    Ok(CrestClearance {
        min_clearance_m,
        min_clearance_downrange_m,
        first_blocking_downrange_m,
    })
}

/// Re-flies a solved charge along its aim, recording every state, and measures its clearance
/// over `profile` along the gun-to-target line of `problem`.
///
/// # Errors
///
/// [`CrestClearanceError::Flight`] when the solved launch no longer flies to the target height;
/// otherwise as [`crest_clearance`].
pub fn crest_clearance_of_charge(
    problem: &ChargeProblem,
    solved: &ChargeElevation,
    gun_height_m: f64,
    profile: &TerrainProfile,
) -> Result<CrestClearance, CrestClearanceError> {
    let launch = Launch {
        muzzle_speed_m_s: problem.muzzle_speed_m_s,
        elevation_rad: solved.elevation_rad,
        azimuth_rad: solved.aim_azimuth_rad,
    };
    let flight = fly_to_height(
        &problem.flight_parameters,
        &launch,
        &problem.wind,
        problem.height_difference_m,
        PathRecording::KeepSamples,
    )
    .map_err(CrestClearanceError::Flight)?;
    crest_clearance(&flight.path, problem.azimuth_rad, gun_height_m, profile)
}

/// The lowest interpolated height of the flight over `downrange_m`, or `None` when no recorded
/// segment spans it.
fn lowest_height_at(flight: &[(f64, f64)], downrange_m: f64) -> Option<f64> {
    let mut lowest: Option<f64> = None;
    for pair in flight.windows(2) {
        let ((start_d, start_h), (end_d, end_h)) = (pair[0], pair[1]);
        let (near_d, far_d) = if start_d <= end_d {
            (start_d, end_d)
        } else {
            (end_d, start_d)
        };
        if downrange_m < near_d || downrange_m > far_d {
            continue;
        }
        let height_m = if start_d == end_d {
            start_h.min(end_h)
        } else if downrange_m == end_d {
            end_h
        } else if downrange_m == start_d {
            start_h
        } else {
            let fraction = (downrange_m - start_d) / (end_d - start_d);
            start_h + fraction * (end_h - start_h)
        };
        lowest = Some(lowest.map_or(height_m, |least| least.min(height_m)));
    }
    lowest
}

fn require(parameter: &'static str, value: f64, valid: bool) -> Result<(), CrestClearanceError> {
    if valid {
        Ok(())
    } else {
        Err(CrestClearanceError::InvalidInput { parameter, value })
    }
}

#[cfg(test)]
#[path = "tests/crest_clearance.rs"]
mod tests;
