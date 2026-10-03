//! The firing solver: from a gun, a target and a catalog shell to an elevation per charge.
//!
//! **Role:** inverts the shell flight model. For each charge of the shell it finds the aim
//! azimuth and high-angle elevation whose flight, drifted by the wind, descends through the
//! target height on the target, refuses with a typed reason when none exists, and recommends the charge with the
//! fewest rings that solves.
//! **Position:** ballistics tier 2, over `ballistics_model`; reads a
//! [`ballistics_model::catalog::BallisticsCatalog`] through its typed lookups
//! and flies [`ballistics_model::flight_model::fly_to_height`]. The API
//! re-solves stored fire missions and the mortar calculator solves locally through
//! [`solve_fire_solution`]; the battery, dispersion, fuze and calibration build on
//! [`solve_charge_elevation`] and the searches of [`elevation_search`].
//! **Signals & state:** none; pure functions over plain values and a borrowed catalog.
//! **Invariants:**
//! - Per charge: `θ*` = argmax of range on `[el_min, el_max]` by golden section
//!   ([`elevation_search::GOLDEN_SECTION_ITERATIONS`] iterations); then the root of
//!   `range(θ) - D` on `[max(θ*, el_min), el_max]` by Brent's method with a bisection fallback,
//!   at most 60 iterations, bracket under 1e-9 rad.
//! - Refusals are [`SolutionRefusal`] values: too close, out of range, unreachable, did not
//!   converge, lifetime exceeded, invalid input. No input, however malformed, panics.
//! - With wind, the aim point is iterated (at most [`wind_corrected_aim::MAX_AIM_ITERATIONS`]
//!   elevation solves) until the impact lies under [`wind_corrected_aim::AIM_MISS_TOLERANCE_M`]
//!   from the target; in calm air the aim is the geometric line, bit for bit.
//! - Every charge is evaluated; the recommended charge is the lowest ring count that solves.
//! - Angles leave in degrees and in the weapon's mils convention.

pub mod charge_selection;
pub mod crest_clearance;
pub mod dispersion;
pub mod elevation_search;
mod error;
pub mod fire_solution;
pub mod prelude;
pub mod wind_corrected_aim;

/// Re-exports the per-charge solve, its row, its refusals and the recommendation.
pub use charge_selection::{
    ChargeElevation, ChargeProblem, ChargeSolution, SolutionRefusal, recommended_rings,
    solve_charge_elevation,
};
/// The crate's error and result.
pub use error::{Error, Result};
/// Re-exports the one-gun request, its solution and its error.
pub use fire_solution::{
    FireSolution, FireSolutionError, FireSolutionRequest, MapPosition, solve_fire_solution,
};

/// The per-charge solver and its searches against the minimal test catalog.
#[cfg(test)]
#[path = "tests/solver.rs"]
mod tests;

/// Rotation, crosswind-mirror and height-difference symmetry of the firing solver.
#[cfg(test)]
#[path = "tests/symmetry.rs"]
mod tests_symmetry;

/// Seeded random requests, clean and corrupted, solved without a panic into typed answers.
#[cfg(test)]
#[path = "tests/bounded_failure.rs"]
mod tests_bounded_failure;
