//! A seeded, deterministic lattice of battery fire problems for native/wasm32 agreement.
//!
//! **Role:** draws battery fire problems over a catalog from a seed ([`agreement_cases`]), maps a
//! case to its fire-mission inputs ([`fire_mission_inputs`]), restates a solution's lead gun
//! ([`lead_summary`]) and records the IEEE 754 bit pattern of every `f64` of a case's inputs
//! and solution ([`case_bit_patterns`]).
//! **Position:** ballistics tier 4, over `ballistics_model`, `ballistics_solver`,
//! `fire_mission_planning` and `deterministic_random`. The developer tools' native agreement
//! gate and the single-page app's URL-only agreement bench draw from the same seed and count.
//! **Signals & state:** none; pure functions over a borrowed catalog.
//! **Invariants:** the same catalog, seed and count give the same cases, inputs and bit patterns
//! on every target.

mod case_lattice;
pub mod ids;
pub mod prelude;

/// The drawn case, the draw, the mapping, the lead summary and the bit walks.
pub use case_lattice::{
    AgreementCase, MAX_GUNS_PER_CASE, MAX_WIND_SPEED_M_S, agreement_cases, case_bit_patterns,
    f64_bit_patterns, fire_mission_inputs, lead_summary,
};
/// The identifier of one drawn case.
pub use ids::AgreementCaseId;
