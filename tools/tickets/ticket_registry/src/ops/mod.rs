//! The typed operations that change ticket files.
//!
//! **Role:** every mutation of the ticket corpus — mint ([`add`], [`add_child`]), status
//! transitions ([`set_status`], [`ship`], [`stamp_sha`], [`mark_ready`]), ordering
//! ([`reorder`], [`advance_slice`]) and deletion ([`remove`]) — as a function over an in-memory
//! [`Corpus`] that reports which files to write and delete in an [`OpOutcome`].
//! **Position:** called by `crate::verbs` after the `ticket check` preflight; the verb hands the
//! outcome to `Corpus::write_back` and `Corpus::delete_files`, then runs `ticket sync`.
//! **Signals & state:** mutates only the `Corpus` it is given; reads the disk only to confirm a
//! spec or plan file exists. The clock arrives as an RFC 3339 UTC argument.
//! **Invariants:** each operation builds a candidate image and validates it as a whole before it
//! touches the corpus, so no operation writes a corpus its own preflight would refuse; a refusal
//! leaves the corpus unchanged and comes back as the exact text the command prints.

use ticket_model::store::Corpus;
use ticket_model::{
    Domain, ProgramTicket, ScopeV2, Status, StatusName, Ticket, TicketId, WorkTicket,
    parse_ticket_toml, render_ticket_toml,
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
