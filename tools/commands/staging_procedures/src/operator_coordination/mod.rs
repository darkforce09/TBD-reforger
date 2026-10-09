//! What the harness tells the orchestrator and the operator: the approved action lists and the
//! awaited effects.
//!
//! **Role:** declares [`action_list`] (the numbered real actions per procedure, its recovery
//! list and its declared cases) and [`awaited_effect`] (the `AWAIT` and outcome lines).
//!
//! **Position:** between the procedures, which supply the lists and steps, and the person or
//! orchestrator who acts on them.
//!
//! **Signals & state:** none; values and pure formatting.
//!
//! **Invariants:** the harness only prints; it never waits for typed input.

pub(crate) mod action_list;
pub(crate) mod awaited_effect;
