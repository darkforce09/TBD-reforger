//! Role: Module boundary for editing/tools.
//! Position: `editing` in the map engine.
//! Signals & state: headless tool state machines driven by explicit world coordinates.
//! Invariants: a tool here holds phase and geometry — armed, active, committed — and never a
//! pointer event, a reactive signal, or an element handle. Hosts inject transports and clocks.

/// Selection: the left-button gesture model, picking, marquee and drag previews.
pub mod selection;

/// The placement vocabulary interactive tools speak.
pub mod placement;

/// Line of sight: the ray, the viewshed disc, and the object layer over both.
pub mod line_of_sight;

/// One live compute job per tool, advanced in budgeted batches and cancelled by the next placement.
pub mod viewshed_scheduler;
