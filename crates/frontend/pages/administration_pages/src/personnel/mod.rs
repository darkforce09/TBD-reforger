//! The personnel roster: who is in the unit, and the standing each member holds.
//!
//! **Role:** declares the route component, the roster address kept in the URL, the roster table and
//! its pager, the dossier pane, and the role and sanction controls the dossier opens.
//! **Position:** the `/admin/personnel` route, in the administration hub.
//! **Signals & state:** none at this level; the page owns the fetch and every shared signal.
//! **Invariants:** both panes and the pager read the same fetched page, so the table, the dossier
//! and the page position can never disagree about who is on the roster.

mod dossier;
mod member_roster;
pub mod page;
mod role_dialog;
mod roster_pager;
mod roster_query;

#[cfg(target_arch = "wasm32")]
pub use page::PersonnelRosterPage;

#[cfg(test)]
use member_roster::{FilterMode, SortMode, apply_roster_filter, apply_roster_sort};
#[cfg(test)]
use page::{ADMIN_ROLES_SYNC_PATH, roles_sync_success_message, roles_sync_updated_count};
#[cfg(test)]
use role_dialog::{
    BanReason, admin_user_ban_path, admin_user_warnings_path, classify_ban_reason,
    reason_confirm_enabled,
};

#[cfg(test)]
#[path = "tests/personnel.rs"]
mod tests;
