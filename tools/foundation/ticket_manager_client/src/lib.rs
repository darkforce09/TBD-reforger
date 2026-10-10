//! The typed client of the central ticket manager's `ttm` command line.
//!
//! **Role:** [`TicketManager`] runs `ttm --json --project <project> …` and parses its versioned
//! JSON documents ([`ticket_documents`], [`wave_documents`]) into typed values; a refusal keeps
//! the ticket manager's error kind ([`Error`]). [`is_ticket_reference`] and [`parent_slice`] are
//! the ticket reference shapes the wave drivers accept from a command line.
//! **Position:** tier 2 of `tools/foundation`, over `process_runner` (the child process),
//! `verification_core` (why a child did not run) and `newtype_ids`. The slice runner, the platform
//! and mod wave drivers and the preflight read and change tickets, receipts and waves through it;
//! no workspace crate reads the legacy ticket files any more.
//! **Signals & state:** none held; every call spawns `ttm` once and inherits the environment
//! (`TBD_TTM_BIN`, `TBD_TTM_PROJECT`, `TBD_TICKETS_DB`).
//! **Invariants:** a document is accepted only with the format tag the contract names for its
//! command; a missing binary is a did-not-run, never a refusal or an empty answer; the client
//! links no code of the ticket manager, only its command line.

mod error;
pub mod prelude;
mod ticket_commands;
pub mod ticket_documents;
mod ticket_manager;
mod ticket_references;
mod wave_commands;
pub mod wave_documents;

pub use error::{Error, NOT_FOUND_KIND, Result};
pub use ticket_commands::{RunRecord, TokenCounts};
pub use ticket_documents::TicketManagerDocument;
pub use ticket_manager::{
    BINARY_VARIABLE, DEFAULT_BINARY, DEFAULT_PROJECT, PROJECT_VARIABLE, TicketManager,
    parse_document,
};
pub use ticket_references::{
    LegacyTicketNumber, ReceiptName, TicketSlug, is_legacy_ticket_number, is_ticket_reference,
    parent_slice,
};
pub use wave_documents::{WavePlan, WaveRow, is_complete_status};

#[cfg(test)]
#[path = "tests/ticket_documents_tests.rs"]
mod ticket_documents_tests;

#[cfg(test)]
#[path = "tests/live_ticket_manager_tests.rs"]
mod live_ticket_manager_tests;
