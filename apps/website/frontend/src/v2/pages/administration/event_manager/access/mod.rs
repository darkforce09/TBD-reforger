//! One operation's access administration: who may see and join it, from which pools, and why each
//! participant is admitted.
//!
//! **Role:** declares the access panel the operations calendar opens on the selected operation —
//! its state and single change path, the editors for the operation, squad and slot policies, the
//! event groups with their rosters, the reservation pools, the participant evidence table and the
//! per-mission waiting-list promotion — and re-exports the panel's handle and sheet to the calendar.
//! **Position:** inside the `/admin/events` route, as a side sheet over the calendar.
//! **Signals & state:** none at this level; [`state::AccessPanel`] holds everything, and the
//! calendar's state creates it.
//! **Invariants:** grants are alternatives and the conditions of a grant are all required; an empty
//! grant list admits nobody, while removing a squad's or slot's own policy makes it inherit — two
//! decisions the panel keeps visibly apart. Every change names the access revision it was prepared
//! against, and a stale revision reloads the view and says so rather than overwriting another
//! administrator's change.

mod change_report;
mod groups;
mod member_search;
mod panel;
mod participants_table;
mod policy_draft;
mod policy_editor;
mod policy_inheritance;
mod policy_lists;
mod quota_editor;
mod state;
mod waitlist_promotion;

pub(super) use panel::access_sheet;
pub(super) use state::AccessPanel;

#[cfg(test)]
#[path = "tests/access_drafts.rs"]
mod draft_tests;

#[cfg(test)]
#[path = "tests/access_evidence.rs"]
mod evidence_tests;
