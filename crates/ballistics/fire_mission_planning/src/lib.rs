//! The fire-mission assembler: a fire mission's inputs against a pinned catalog to its solution.
//!
//! **Role:** solves every gun of a battery onto one target ([`battery`]), the time-fuze setting
//! of each charge ([`fuze`]), and assembles them with crest clearance and dispersion into one
//! [`FireMissionSolution`] ([`fire_mission`]); compares two solutions under the client/server
//! mismatch rule ([`fire_mission_comparison`]) and words a solution for display
//! ([`solution_wording`]).
//! **Position:** ballistics tier 3, over `ballistics_model` and `ballistics_solver`. The API
//! re-solves and compares every submitted fire mission, the mortar calculator solves locally,
//! and the developer tools' browser gates and the agreement cases solve through
//! [`solve_fire_mission`].
//! **Signals & state:** none; pure functions over plain values and a borrowed catalog.
//! **Invariants:** inputs and solutions match `contracts/definitions/fire-mission.schema.json`;
//! [`SOLVER_REVISION`] names the solver every stored solution was computed by; a solution is a
//! pure function of its inputs and catalog, with the same bits on native and wasm32 builds.

pub mod battery;
mod error;
pub mod fire_mission;
pub mod fire_mission_comparison;
pub mod fuze;
pub mod prelude;
pub mod solution_wording;

/// The crate's error and result.
pub use error::{Error, Result};
/// The fire mission's inputs, solution, refusal, solver revision and the assembler.
pub use fire_mission::{
    FireMissionInputs, FireMissionRefusal, FireMissionSolution, SOLVER_REVISION, solve_fire_mission,
};
/// The client/server mismatch rule.
pub use fire_mission_comparison::compare_solutions;

/// The vanilla catalog through the fire-mission assembler for every weapon and shell.
#[cfg(test)]
#[path = "tests/end_to_end.rs"]
mod tests_end_to_end;
