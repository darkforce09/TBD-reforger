//! Ops for the ticket domain.

use crate::store::Corpus;
use crate::{
    Domain, ProgramTicket, ScopeV2, Status, StatusName, Ticket, WorkTicket, parse_ticket_toml,
    render_ticket_toml,
};

use std::collections::{BTreeMap, BTreeSet};

mod fields;

pub use fields::{OpOutcome, VALID_STATUS_NAMES};

use fields::{
    acceptance_of, created_at_of, current_shipped_at, depends_on_of, live_order_sets, main_goal_of,
    plan_of, set_acceptance, set_completed_at, set_main_goal, set_plan, set_spec,
    set_ticket_status, spec_of, summary_of, title_of, unknown, validate_clock,
};

mod validation;

use validation::validate_post_image;

mod transitions;

pub use transitions::{set_status, ship, stamp_sha};

use transitions::commit;

mod readiness;

pub use readiness::{default_plan_path, mark_ready};

mod creation;

pub use creation::{add, add_child};

mod ordering;

pub use ordering::{advance_slice, remove, reorder};

use ordering::append_order;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
