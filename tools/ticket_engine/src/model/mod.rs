//! Typed ticket model and field contracts.

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
