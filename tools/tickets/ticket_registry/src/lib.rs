//! The ticket registry: its JSON projection, the typed operations, the derived files, the checks
//! and the ticket command verbs.
//!
//! **Role:** projects the ticket files into one JSON value ([`registry`]), changes ticket files
//! through validated typed operations ([`ops`]), regenerates `queue.json`, the roadmap block and
//! the gap-analysis column ([`sync`]), runs `ticket check` and the mutation preflight
//! ([`validation`]), reads the corpus pins ([`corpus_pins`]) and holds the body of every
//! `cargo xtask ticket` verb ([`verbs`]).
//! **Position:** tier 4 of `tools/tickets`, over `ticket_model`, `ticket_metrics` and
//! `ticket_wave_lock`; the xtask `ticket`, `wave`, `platform` and `mod` command groups call it.
//! **Signals & state:** none; every function takes the checkout root and reads or writes files.
//! **Invariants:** a mutation refuses on a red check and writes nothing; the typed operations
//! are the only writer of ticket files; the crate never starts an agent or deletes a worktree.

pub mod corpus_pins;
mod error;
pub mod ops;
pub mod prelude;
pub mod registry;
pub mod sync;
pub mod validation;
pub mod verbs;

pub use error::{Error, Result};
pub use ops::OpOutcome;
pub use registry::load_registry;
