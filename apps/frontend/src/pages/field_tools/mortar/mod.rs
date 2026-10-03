//! The mortar calculator page: the catalog, the inputs, the map, the local solve, the solution
//! and the saved fire missions.
//!
//! **Role:** declares the route component, the catalog source, the input groups, the map picker,
//! the solve bridge to the ballistics crates' solver, the solution panel, the offline pack line and the
//! save area, and re-exports the page for the router.
//! **Position:** the public `/tools/mortar` route, in the field-tools hub.
//! **Signals & state:** none at this level; the page owns every signal and every fetch.
//! **Invariants:** the firing solution is computed on this device by the ballistics crates' solver
//! against a catalog read from the public catalog routes (or the offline copy of them); the API
//! re-solves a saved fire mission with the same solver and refuses one that differs.

mod catalog_source;
mod inputs;
mod map_picker;
mod offline_status;
mod page;
mod saved_fires;
mod solution;
mod solve_bridge;

#[cfg(target_arch = "wasm32")]
pub use page::MortarCalculatorPage;

#[cfg(test)]
#[path = "tests/test_mission.rs"]
mod test_mission;
