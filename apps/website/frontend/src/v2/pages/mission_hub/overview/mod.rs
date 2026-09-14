//! The mission overview page: one mission's dossier, and the armory editor behind it.
//!
//! **Role:** declares the route component, the dossier header, the shared read-only body and its
//! briefing section, and the Edit Armory dialog with the draft state and guards it is built on.
//! **Position:** the `/missions/:id` route, in the mission hub.
//! **Signals & state:** none at this level; the page owns the fetch, and the armory editor handle
//! carries every signal the dialog needs.
//! **Invariants:** the dossier body is shared with the library's slide-over, so it stays
//! read-only — the write half lives in the dialog, which only this route can open.
#![allow(dead_code)]

mod armory_dialog;
mod armory_editor;
pub mod dossier_body;
mod header;
mod intel_briefing;
mod page;

pub use dossier_body::dossier_body;
pub(crate) use dossier_body::mission_status_label;
pub use page::MissionOverviewPage;

// The test battery spans every shard of this page, so the names it reaches for through
// `use super::*` are gathered here.
#[cfg(test)]
use crate::v2::core::api::dto::MissionDetail;
#[cfg(test)]
use armory_editor::{
    armory_body, draft_problem, key_storable, orbat_faction_keys, parse_qty, DraftRow,
};
#[cfg(test)]
use dossier_body::{detail_rows, tactical_briefing_text};

#[cfg(test)]
#[path = "tests/mission_overview.rs"]
mod tests;
