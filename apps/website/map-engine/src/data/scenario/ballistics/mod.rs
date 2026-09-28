//! Role: Module boundary for data/scenario/ballistics.
//! Position: `data/scenario/ballistics` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Point-mass shell flight: the engine's 1/30 s step with quadratic drag, crossing and apex.
pub mod flight_model;

/// Constant surface wind and its air-velocity vector.
pub mod wind;

/// Degrees, radians and weapon mils; azimuth normalisation.
pub mod angular_units;

/// Game ballistics catalog: weapons, shells, charges and typed lookups.
pub mod catalog;

/// Firing solver: high-angle elevation per charge, refusals and the recommended charge.
pub mod solver;

/// Impact dispersion of a solved charge: probable errors and the 50 % ellipse.
pub mod dispersion;

/// Time-fuze setting of a charge for a burst above the target.
pub mod fuze;

/// A battery: every gun solved independently onto one target, with its dispersion.
pub mod battery;

/// Trajectory clearance over a terrain profile along the line of fire.
pub mod crest_clearance;

/// A seeded, deterministic lattice of battery cases for native/wasm32 agreement.
pub mod agreement_cases;

/// The one fire-mission assembler: inputs against a pinned catalog to the whole solution.
pub mod fire_mission;

/// The client/server mismatch rule between two fire-mission solutions.
pub mod fire_mission_comparison;

/// Calibration of a catalog against the game's tables and the engine oracle.
pub mod calibration;

/// The words a fire-mission solution is shown in, shared by every surface that shows or checks it.
pub mod solution_wording;

/// Re-exports the one-gun firing solution entry point, its request, result and error.
pub use solver::{FireSolution, FireSolutionError, FireSolutionRequest, solve_fire_solution};

/// Rotation, crosswind-mirror and height-difference symmetry of the firing solver.
#[cfg(test)]
#[path = "tests/symmetry.rs"]
mod tests_symmetry;

/// The firing solver inverted over the engine oracle's height and wind samples.
#[cfg(test)]
#[path = "tests/oracle_elevation_and_wind.rs"]
mod tests_oracle_elevation_and_wind;

/// Seeded random requests, clean and corrupted, solved without a panic into typed answers.
#[cfg(test)]
#[path = "tests/bounded_failure.rs"]
mod tests_bounded_failure;

/// The vanilla catalog through the fire-mission assembler for every weapon and shell.
#[cfg(test)]
#[path = "tests/end_to_end.rs"]
mod tests_end_to_end;
