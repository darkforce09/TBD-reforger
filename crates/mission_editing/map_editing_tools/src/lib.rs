//! The Mission Creator's headless interactive map tools.
//!
//! **Role:** holds the state machines, geometry and verdicts of the Mission Creator's map tools:
//! the left-button gesture model with its picks, marquees and index self-checks ([`selection`]),
//! the session-local ruler polyline ([`ruler`]), the sight ray and viewshed disc with the object
//! layer over both ([`line_of_sight`]), and the scheduler that runs one budgeted, cancellable
//! visibility job per tool ([`viewshed_scheduler`]).
//! **Position:** mission editing category, tier 7, over `mission_editing_session` (the picks joined
//! to document ids), `mission_document`, `mission_crdt`, `camera_math`, `spatial_indexes`, the
//! three line-of-sight crates, `terrain_elevation` and `time_source`. The Mission Creator in
//! `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/` installs the session cells and the scheduler
//! host, feeds the tools world coordinates and draws what they hold.
//! **Signals & state:** thread-local host cells per tool (the ruler chain, the line-of-sight
//! capture, the DEM sampler, the viewshed state), the scheduler's service table, its two job slots
//! and its cancel token; everything else is pure over its arguments.
//! **Invariants:** no browser, UI framework or GPU type crosses into this crate, and no tool holds
//! a pointer event, a reactive signal or an element handle; ruler and line-of-sight results are
//! measurements that never reach the mission document; selection is app state.

pub mod line_of_sight;
pub mod prelude;
pub mod ruler;
pub mod selection;
pub mod viewshed_scheduler;
