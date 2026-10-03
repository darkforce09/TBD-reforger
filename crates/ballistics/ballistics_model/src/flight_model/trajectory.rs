//! The flight loop: from the muzzle to the descending crossing of a target height.
//!
//! **Role:** validates the inputs, steps the shell with the engine's fixed step until it falls
//! through the target height, and reports the crossing, the apex and, on request, every step's
//! state.
//!
//! **Position:** the `ballistics_model` crate's `flight_model` module; [`fly_to_height`] is the module's entry point, called
//! by the firing solver, fuze, crest and calibration computations; it flies
//! [`fly_to_height_in`] in `f32`, the engine's precision. [`fly_to_height_double_precision`]
//! flies the same scheme in `f64` for the dispersion's derivatives. It drives
//! `integrator::engine_step`
//! and `crossing`'s linear crossing.
//!
//! **Signals & state:** none; each call owns its state and an optional sample buffer.
//!
//! **Invariants:** [`fly_to_height`] runs in `f32` (state, constants, target height, crossing)
//! and widens to `f64` only in [`FlightOutcome`] and [`FlightSample`]; times are the exact step
//! count plus the crossing fraction times the rounded step, formed in `f64`. The step count
//! never exceeds `ceil(time_to_live_s / integration_step_s)`; a crossing after `time_to_live_s`
//! is refused like no crossing at all; the apex is the highest step point of the polyline the
//! crossing interpolates, so a target above it is never crossed; once the vertical velocity is
//! non-positive it stays so (wind is horizontal, drag opposes motion), so a shell descending
//! below the target height can never reach it and is refused at once.

use super::crossing::{StepEndpoints, descending_crossing_fraction};
use super::integrator::{FlightConstants, ShellState, engine_step};
use super::precision::FlightFloat;
use super::{FlightError, FlightParameters, Launch};
use crate::wind::Wind;

/// Whether [`fly_to_height`] keeps the state of every integration step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathRecording {
    /// Keep only the summary; [`FlightOutcome::path`] stays empty.
    Discard,
    /// Keep the muzzle state, every step's end state and the crossing state.
    KeepSamples,
}

/// The shell's state at one instant of a recorded flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightSample {
    /// Seconds since firing.
    pub time_s: f64,
    /// `[east, north, up]` metres from the muzzle.
    pub position_m: [f64; 3],
    /// `[east, north, up]` metres per second.
    pub velocity_m_s: [f64; 3],
}

/// Where and when the shell falls through the target height.
#[derive(Debug, Clone, PartialEq)]
pub struct FlightOutcome {
    /// Seconds from firing to the descending crossing.
    pub time_of_flight_s: f64,
    /// `[east, north, up]` metres from the muzzle at the crossing; `up` equals the target height
    /// as the flight holds it (rounded to `f32` by [`fly_to_height`]).
    pub impact_position_m: [f64; 3],
    /// `[east, north, up]` metres per second at the crossing.
    pub impact_velocity_m_s: [f64; 3],
    /// Distance along the launch azimuth to the crossing, metres.
    pub downrange_m: f64,
    /// Distance to the right of the launch azimuth at the crossing, metres (left is negative).
    pub deflection_m: f64,
    /// Highest step point of the flight, metres above the muzzle.
    pub apex_height_m: f64,
    /// Seconds from firing to the highest step point.
    pub apex_time_s: f64,
    /// Recorded states in time order, ending at the crossing; empty for
    /// [`PathRecording::Discard`].
    pub path: Vec<FlightSample>,
}

/// Flies a shell from the muzzle until it descends through `target_height_m` (metres above the
/// muzzle), in `f32` like the game engine.
///
/// # Errors
///
/// [`FlightError::InvalidInput`], [`FlightError::InvalidWind`] or
/// [`FlightError::TooManyIntegrationSteps`] for inputs outside their domain;
/// [`FlightError::TargetAboveApex`] when the shell starts descending below the target height;
/// [`FlightError::TimeToLiveExceeded`] when no crossing happens within the shell lifetime.
pub fn fly_to_height(
    parameters: &FlightParameters,
    launch: &Launch,
    wind: &Wind,
    target_height_m: f64,
    recording: PathRecording,
) -> Result<FlightOutcome, FlightError> {
    fly_to_height_in::<f32>(parameters, launch, wind, target_height_m, recording)
}

/// [`fly_to_height`] in `f64`: the same engine step, step size, constants and crossing, each
/// held in double precision, so the point of fall is a smooth function of the launch. Its
/// point of fall lies within 0.02 m of [`fly_to_height`]'s; derivatives of the impact point
/// (the dispersion's finite differences) use it, every reproduction of the engine uses
/// [`fly_to_height`].
///
/// # Errors
///
/// As [`fly_to_height`].
pub fn fly_to_height_double_precision(
    parameters: &FlightParameters,
    launch: &Launch,
    wind: &Wind,
    target_height_m: f64,
    recording: PathRecording,
) -> Result<FlightOutcome, FlightError> {
    fly_to_height_in::<f64>(parameters, launch, wind, target_height_m, recording)
}

/// [`fly_to_height`] in the precision `F`: `f32` for every production flight, `f64` for the
/// proofs of the scheme's own properties free of single-precision rounding.
///
/// # Errors
///
/// As [`fly_to_height`].
pub(super) fn fly_to_height_in<F: FlightFloat>(
    parameters: &FlightParameters,
    launch: &Launch,
    wind: &Wind,
    target_height_m: f64,
    recording: PathRecording,
) -> Result<FlightOutcome, FlightError> {
    parameters.validate()?;
    launch.validate()?;
    wind.validate()?;
    if !target_height_m.is_finite() {
        return Err(FlightError::InvalidInput {
            parameter: "target_height_m",
            value: target_height_m,
        });
    }

    let constants = FlightConstants::<F>::new(
        parameters.gravity_m_s2,
        parameters.air_drag,
        parameters.mass_kg,
        parameters.wind_influence_multiplier,
        wind.air_velocity_m_s(),
    );
    let step_s = F::from_f64(parameters.integration_step_s);
    let step_limit = (parameters.time_to_live_s / parameters.integration_step_s).ceil() as u64;
    let target_m = F::from_f64(target_height_m);
    let elapsed_s = |steps: u64, fraction: F| (steps as f64 + fraction.to_f64()) * step_s.to_f64();

    let mut state = ShellState {
        position_m: [F::ZERO; 3],
        velocity_m_s: launch.velocity_in::<F>(),
    };
    let mut apex = (F::ZERO, 0.0_f64);
    let mut path = Vec::new();
    let mut record = |time_s: f64, sample: ShellState<F>| {
        if recording == PathRecording::KeepSamples {
            path.push(FlightSample {
                time_s,
                position_m: sample.position_m.map(F::to_f64),
                velocity_m_s: sample.velocity_m_s.map(F::to_f64),
            });
        }
    };
    record(0.0, state);

    for step_index in 0..step_limit {
        let next = engine_step(&constants, state, step_s);
        let step = StepEndpoints {
            start: state,
            end: next,
        };
        if next.position_m[2] > apex.0 {
            apex = (next.position_m[2], elapsed_s(step_index + 1, F::ZERO));
        }
        let z0 = state.position_m[2];
        let z1 = next.position_m[2];
        if z0 >= target_m && z1 < target_m && next.velocity_m_s[2] < F::ZERO {
            let s = descending_crossing_fraction(&step, target_m);
            let time_of_flight_s = elapsed_s(step_index, s);
            if time_of_flight_s > parameters.time_to_live_s {
                break;
            }
            let mut impact_position_m = step.position_at(s);
            impact_position_m[2] = target_m;
            let impact = ShellState {
                position_m: impact_position_m,
                velocity_m_s: step.velocity_at(s),
            };
            record(time_of_flight_s, impact);
            let apex = (apex.0.to_f64(), apex.1);
            return Ok(outcome(launch, impact, time_of_flight_s, apex, path));
        }
        if next.velocity_m_s[2] <= F::ZERO && z1 < target_m {
            return Err(FlightError::TargetAboveApex {
                apex_height_m: apex.0.to_f64(),
                target_height_m,
            });
        }
        record(elapsed_s(step_index + 1, F::ZERO), next);
        state = next;
    }
    Err(FlightError::TimeToLiveExceeded {
        time_to_live_s: parameters.time_to_live_s,
    })
}

fn outcome<F: FlightFloat>(
    launch: &Launch,
    impact: ShellState<F>,
    time_of_flight_s: f64,
    apex: (f64, f64),
    path: Vec<FlightSample>,
) -> FlightOutcome {
    let (sin_azimuth, cos_azimuth) = (libm::sin(launch.azimuth_rad), libm::cos(launch.azimuth_rad));
    let impact_position_m = impact.position_m.map(F::to_f64);
    let [east, north, _] = impact_position_m;
    FlightOutcome {
        time_of_flight_s,
        impact_position_m,
        impact_velocity_m_s: impact.velocity_m_s.map(F::to_f64),
        downrange_m: east * sin_azimuth + north * cos_azimuth,
        deflection_m: east * cos_azimuth - north * sin_azimuth,
        apex_height_m: apex.0,
        apex_time_s: apex.1,
        path,
    }
}
