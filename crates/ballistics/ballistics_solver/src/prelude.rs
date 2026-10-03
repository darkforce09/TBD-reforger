//! The names a caller of the firing solver imports with `use ballistics_solver::prelude::*;`.

pub use crate::charge_selection::{ChargeSolution, SolutionRefusal, recommended_rings};
pub use crate::crest_clearance::{CrestClearance, TerrainProfile, TerrainSample};
pub use crate::dispersion::ImpactDispersion;
pub use crate::fire_solution::{
    FireSolution, FireSolutionError, FireSolutionRequest, MapPosition, solve_fire_solution,
};
