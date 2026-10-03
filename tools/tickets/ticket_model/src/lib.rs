//! The typed ticket and the files that hold it.
//!
//! **Role:** the [`Ticket`] model (kinds, statuses, scope, field caps and shared predicates), the
//! [`TicketId`] newtype, the canonical TOML encoding ([`TicketFile`], [`parse_ticket_toml`],
//! [`render_ticket_toml`]), the [`Corpus`] store over `.ai/tickets/T-*.toml`, the
//! [`ScopeVocab`] word list, the repository paths only the ticket domain names
//! ([`repository`]) and the commit-subject miner ([`commit_subjects`]).
//! **Position:** tier 2 of `tools/tickets`, over `repository_layout`, `time_source`,
//! `process_runner` and `newtype_ids`; `ticket_metrics`, `ticket_wave_lock`, `ticket_registry`,
//! xtask and the ticketboard read tickets through it.
//! **Signals & state:** none; every function takes the checkout root and reads or writes files.
//! **Invariants:** every ticket file loads and renders back byte for byte; a malformed timestamp
//! or an illegal field value refuses the parse; a write touches only the ids it is given.

pub mod commit_subjects;
mod encoding;
mod error;
mod error_chain;
mod model;
pub mod prelude;
pub mod repository;
pub mod store;
mod ticket_id;
pub mod vocab;

#[cfg(test)]
#[path = "tests/proptest_roundtrip_tests.rs"]
mod proptest_roundtrip_tests;

pub use encoding::{TicketFile, parse_ticket_toml, render_ticket_toml};
pub use error::{Error, Result};
pub use error_chain::error_chain_text;
pub use model::{
    BODY_LINE_WORD_CAP, CITATION_WORD_CAP, CLASS_VALUES, Domain, ESTIMATED_VALUES,
    MAIN_GOAL_DEBT_PIN, ProgramTicket, SUMMARY_WORD_CAP, ScopeV2, Status, StatusName,
    TITLE_DEBT_PIN, TITLE_WORD_CAP, Ticket, WorkTicket, classify_work, empty_ready_tier_fields,
    is_sha_shaped, main_goal_is_debt, title_is_debt,
};
pub use store::Corpus;
pub use ticket_id::TicketId;
pub use vocab::ScopeVocab;
