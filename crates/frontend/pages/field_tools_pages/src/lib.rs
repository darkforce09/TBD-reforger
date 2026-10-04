//! The field tools: the standalone tactical aids, today the mortar calculator.
//!
//! **Role:** groups the calculators and inspectors that stand on their own rather than hanging off
//! a mission or an operation.
//! **Position:** a page crate above the foundation crates and the ballistics, map coordinate,
//! overlay and line-of-sight crates; the app's route table mounts its `/tools/…` routes.
//! **Signals & state:** none at this level; each page owns its own.
//! **Invariants:** nothing here is required by another page crate — a page in this crate can be
//! removed without touching the rest of the tree. The route component and every panel that
//! fetches or touches the map are compiled for `wasm32` only, because the endpoints and the map
//! engine they reach exist only in the browser build.

pub mod mortar;
pub mod prelude;
