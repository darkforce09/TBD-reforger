//! The typed ticket and the rules on its fields.
//!
//! **Role:** the ticket kinds ([`Ticket`], [`ProgramTicket`], [`WorkTicket`]), the status that
//! carries each status's fields ([`Status`]), the scope ([`ScopeV2`], [`Domain`]), and the value
//! sets, word caps and predicates the checks and the operations share.
//! **Position:** the bottom of `ticket_model`; the encoding, the store and the vocabulary build on
//! it, and the crate root re-exports every item.
//! **Signals & state:** none; plain data, constants and pure functions.
//! **Invariants:** an illegal status and field combination cannot be built; each rule has one
//! definition here, which every caller uses.

use serde::{Deserialize, Serialize};

mod scope;

pub use scope::{
    BODY_LINE_WORD_CAP, CITATION_WORD_CAP, CLASS_VALUES, Domain, ESTIMATED_VALUES,
    MAIN_GOAL_DEBT_PIN, SUMMARY_WORD_CAP, ScopeV2, TITLE_DEBT_PIN, TITLE_WORD_CAP, classify_work,
    empty_ready_tier_fields, is_sha_shaped, main_goal_is_debt, title_is_debt,
};

mod status;

pub use status::{Status, StatusName};

mod tickets;

pub use tickets::{ProgramTicket, Ticket, WorkTicket};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
