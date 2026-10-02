//! The engine's shell simulation step.
//!
//! **Role:** one fixed step of the shell state exactly as the game engine advances a shell:
//! gravity first, then quadratic air drag on the air-relative velocity, then the position by
//! the mean of the old and new velocities, every operation in the flight's precision (`f32`,
//! the engine's, for every production flight).
//!
//! **Position:** `ballistics/flight_model`; the flight loop builds the [`FlightConstants`] once
//! per flight with [`FlightConstants::new`] and calls [`engine_step`] once per fixed step.
//!
//! **Signals & state:** none; pure functions over copied values.
//!
//! **Invariants:**
//! - The constants are held as the engine holds them: gravity, the step and the scaled air
//!   velocity are each rounded to the flight's precision once, and the drag per unit mass
//!   `k = air_drag / mass_kg` is divided in that precision.
//! - One step of length `Δt` from `(x, v)`: `u = v - g·Δt·ẑ`; `r = u - w`;
//!   `v' = w + r · (1 - k·|r|·Δt)`; `x' = x + (v + v') · (Δt/2)`. The step is first order in
//!   `Δt`: halving the step halves the distance to the exact flight.
//! - In vacuum the vertical velocity falls by `g·Δt` per step and the mean-velocity position
//!   update integrates that linear velocity exactly, so the step points lie on the parabola up
//!   to rounding.
//! - Only `+ - × / sqrt` are used, never fused, so the step is bit-identical on every IEEE
//!   target.

use super::precision::FlightFloat;

/// Position and velocity of the shell in the flight's precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ShellState<F> {
    /// `[east, north, up]` metres from the muzzle.
    pub position_m: [F; 3],
    /// `[east, north, up]` metres per second.
    pub velocity_m_s: [F; 3],
}

/// Gravity, drag per unit mass and the effective air velocity of one flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FlightConstants<F> {
    /// Gravitational acceleration, positive downwards.
    pub gravity_m_s2: F,
    /// `air_drag / mass_kg`, divided in the flight's precision.
    pub drag_per_mass: F,
    /// Wind velocity already scaled by the shell's wind influence multiplier.
    pub air_velocity_m_s: [F; 3],
}

impl<F: FlightFloat> FlightConstants<F> {
    /// The constants as the engine holds them: every input rounded once, the drag per unit mass
    /// and the scaled air velocity formed in the flight's precision.
    pub(super) fn new(
        gravity_m_s2: f64,
        air_drag: f64,
        mass_kg: f64,
        wind_influence_multiplier: f64,
        air_velocity_m_s: [f64; 3],
    ) -> Self {
        let multiplier = F::from_f64(wind_influence_multiplier);
        Self {
            gravity_m_s2: F::from_f64(gravity_m_s2),
            drag_per_mass: F::from_f64(air_drag) / F::from_f64(mass_kg),
            air_velocity_m_s: air_velocity_m_s.map(|component| multiplier * F::from_f64(component)),
        }
    }
}

/// Advances `state` by `step_s` seconds the way the game engine does.
pub(super) fn engine_step<F: FlightFloat>(
    constants: &FlightConstants<F>,
    state: ShellState<F>,
    step_s: F,
) -> ShellState<F> {
    let old_velocity = state.velocity_m_s;
    let mut velocity = old_velocity;
    velocity[2] -= constants.gravity_m_s2 * step_s;
    let air = constants.air_velocity_m_s;
    let relative: [F; 3] = std::array::from_fn(|axis| velocity[axis] - air[axis]);
    let retained = F::ONE - constants.drag_per_mass * norm(relative) * step_s;
    let half_step_s = F::HALF * step_s;
    let mut next = state;
    for axis in 0..3 {
        velocity[axis] = air[axis] + relative[axis] * retained;
        next.position_m[axis] += (old_velocity[axis] + velocity[axis]) * half_step_s;
    }
    next.velocity_m_s = velocity;
    next
}

/// Euclidean length of a vector.
fn norm<F: FlightFloat>(vector: [F; 3]) -> F {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).square_root()
}
