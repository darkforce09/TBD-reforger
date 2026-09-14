//! The global ladders page and the two surfaces it is built from.
//!
//! **Role:** declares the route component, the podium and roster board, and the slide-over
//! operator dossier, and re-exports the page for the router.
//! **Position:** the `/leaderboards` route, in the operations hub.
//! **Signals & state:** none at this level; the page owns the controls and the board fetch, and
//! the dossier owns its own.
//! **Invariants:** ordering and filtering happen on the server, so nothing here re-sorts a page
//! of rows and calls it a ranking.
#![allow(dead_code)]

mod board_table;
mod operator_dossier;
mod page;

pub use page::LeaderboardsPage;

// The guard battery spans every shard of this page, so the names it reaches for through
// `use super::*` are gathered here.
#[cfg(test)]
use board_table::{avatar_img_src, initials, stat_for};
#[cfg(test)]
use page::{parse_row, win_rate_pct};
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
#[path = "tests/leaderboards.rs"]
mod tests;
