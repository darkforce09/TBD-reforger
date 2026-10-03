//! The mission validation rules.
//!
//! **Role:** checks a mission editor payload against an ordered list of
//! rules and answers every finding of every rule ([`default_registry`],
//! [`Registry::evaluate_with_context`], [`validate_editor_payload`]); proves each rule can fire on
//! its own trip fixture ([`Registry::self_check`]).
//! **Position:** mission tier 3, over `mission_payload` (`terrain_bounds`) and
//! `mission_wire_safety` (the cargo capacity scan). The Mission Creator's validation panel
//! evaluates the payload it compiles; the game-document compiler of `mission_compiler` reports its
//! compile findings as [`Finding`] values; the API re-exports [`Finding`] and [`Severity`] for its
//! compile and artifact code.
//! **Signals & state:** none; pure functions over a payload and the facts the caller supplies.
//! **Invariants:** evaluation never stops at the first finding and never panics on a malformed
//! payload; a rule whose fact is absent from the [`EvalContext`] stays inactive; rule ids are
//! distinct within a registry.

mod assets;
mod cargo;
mod context;
mod error;
mod ids;
mod loadout;
mod mission_shape;
mod orbat;
pub mod prelude;
mod registry;
mod rules;

/// The facts a context-dependent rule reads, the loadout policy among them.
pub use context::{EvalContext, LoadoutPolicy};
/// One finding, its severity and the primitive its rule instantiates.
pub use context::{Finding, Primitive, Severity};
/// Why a registry self-check fails, and its result.
pub use error::{Error, Result};
/// The identifiers of a rule, a finding's subject and a placed asset.
pub use ids::{AssetId, RuleId, SubjectId};
/// A rule, the ordered registry of rules, and a rule that failed the self-check.
pub use registry::{Registry, Rule, SelfCheckFailure};
/// The default rule list and the one-call evaluation of an editor payload.
pub use rules::{default_registry, validate_editor_payload};

#[cfg(test)]
mod tests;
