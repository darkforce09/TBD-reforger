//! Role: Module boundary for data/scenario.
//! Position: `data/scenario` in the map engine.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Authored and compiled mission structures.
pub mod ast;

/// Payload and game-document compilation.
pub mod compiler;

/// Authored mission extensions.
pub mod extensions;

/// Validation registry and wire-safety checks.
pub mod validation;

/// Plain-text ORBAT slot formatting.
pub mod slot_line;

/// Mortar ballistics: charge tables and firing solutions.
pub mod ballistics;

/// Expose ast :: factions as orbat at this domain boundary.
pub use ast::factions as orbat;
/// Expose compiler :: { compiler as compile , flatten , kit } at this domain boundary.
pub use compiler::{flatten, kit, payload as compile};
/// Expose extensions :: { environment :: { audio , weather } , modules as spawn modules , objectives :: { tasks , win conditions } , radio as radio plan , tactical graphics , } at this domain boundary.
pub use extensions::{
    environment::{audio, weather},
    modules as spawn_modules,
    objectives::{tasks, win_conditions},
    radio as radio_plan, tactical_graphics,
};
/// Expose validation :: { validator as validate , wire safety } at this domain boundary.
pub use validation::{validator as validate, wire_safety};
