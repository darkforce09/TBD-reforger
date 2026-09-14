//! The personnel roster: who is in the unit, and the standing each member holds.
//!
//! **Role:** declares the route component, the roster table, the dossier pane, and the role and
//! sanction controls the dossier opens.
//! **Position:** the `/admin/personnel` route, in the administration hub.
//! **Signals & state:** none at this level; the page owns the fetch and every shared signal.
//! **Invariants:** both panes read the same fetched page, so the table and the dossier can never
//! disagree about who is on the roster.
#![allow(dead_code)]

mod dossier;
mod member_roster;
mod page;
mod role_dialog;

pub use page::PersonnelRosterPage;

#[cfg(test)]
use member_roster::{apply_roster_filter, apply_roster_sort, FilterMode, SortMode};
#[cfg(test)]
use page::{roles_sync_success_message, roles_sync_updated_count, ADMIN_ROLES_SYNC_PATH};
#[cfg(test)]
use role_dialog::{
    admin_user_ban_path, admin_user_warnings_path, classify_ban_reason, reason_confirm_enabled,
    BanReason,
};

#[cfg(test)]
#[path = "tests/personnel.rs"]
mod tests;
