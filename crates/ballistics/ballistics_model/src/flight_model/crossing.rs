//! Sub-step location of the descending crossing of a target height.
//!
//! **Role:** linear interpolation along one simulation step, the rule the game engine uses to
//! place the end of a shell's flight between two step points, in the flight's precision.
//!
//! **Position:** the `ballistics_model` crate's `flight_model` module; the flight loop hands the endpoints of the step
//! that falls through the target height in a [`StepEndpoints`], asks for
//! [`descending_crossing_fraction`], then evaluates [`StepEndpoints::position_at`] and
//! [`StepEndpoints::velocity_at`] there.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** the flown path is the polyline through the step points; the crossing lies on
//! the chord of its step, at the fraction where the chord's height equals the target height;
//! the fraction lies in `[0, 1]` whenever the start is at or above the target and the end below.

use super::integrator::ShellState;
use super::precision::FlightFloat;

/// The two ends of one simulation step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct StepEndpoints<F> {
    /// State at the start of the step.
    pub start: ShellState<F>,
    /// State at the end of the step.
    pub end: ShellState<F>,
}

impl<F: FlightFloat> StepEndpoints<F> {
    /// Position on the step's chord at fraction `s ∈ [0, 1]`.
    pub(super) fn position_at(&self, s: F) -> [F; 3] {
        std::array::from_fn(|axis| lerp(self.start.position_m[axis], self.end.position_m[axis], s))
    }

    /// Velocity at fraction `s` of the step, interpolated linearly between the step's states.
    pub(super) fn velocity_at(&self, s: F) -> [F; 3] {
        std::array::from_fn(|axis| {
            lerp(
                self.start.velocity_m_s[axis],
                self.end.velocity_m_s[axis],
                s,
            )
        })
    }
}

/// Fraction of the step at which the chord's height falls through `height_m`.
///
/// Requires `start` at or above `height_m` and `end` below it; the answer lies in `[0, 1]`.
pub(super) fn descending_crossing_fraction<F: FlightFloat>(
    step: &StepEndpoints<F>,
    height_m: F,
) -> F {
    let z0 = step.start.position_m[2];
    let z1 = step.end.position_m[2];
    let fraction = (z0 - height_m) / (z0 - z1);
    if fraction < F::ZERO {
        F::ZERO
    } else if fraction > F::ONE {
        F::ONE
    } else {
        fraction
    }
}

fn lerp<F: FlightFloat>(start: F, end: F, s: F) -> F {
    start + (end - start) * s
}
