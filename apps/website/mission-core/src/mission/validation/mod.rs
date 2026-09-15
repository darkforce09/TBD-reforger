//! Role: Module boundary for mission/validation.
//! Position: `mission/validation` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Validation registry, context, and rules.
pub mod validator;

/// Wire-safe strings and cargo capacity checks.
pub mod wire_safety;
