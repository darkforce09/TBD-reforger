//! The mortar calculator page: the coordinate entry, the solve request, and the panels that show
//! and store the answer.
//!
//! **Role:** declares the route component, the two pickers, the coordinate fields and the terrain
//! preview, the firing-solution card, the saved-fire-missions panel and the pure grid maths they
//! all share, and re-exports the page for the router.
//! **Position:** the `/tools/mortar` route, in the field-tools hub.
//! **Signals & state:** none at this level; the page owns every signal and both fetches.
//! **Invariants:** the ballistics are solved on the server — nothing in this folder computes a
//! firing solution, it only sends the geometry and renders what comes back.
#![allow(dead_code)]

mod firing_solution;
mod grid;
mod map_picker;
mod page;
mod saved_fires;
mod weapon_selector;

pub use page::MortarCalculatorPage;

// The test battery spans every shard of this page, so the names it reaches for through
// `use super::*` are gathered here.
#[cfg(test)]
use crate::v2::core::api::dto::{DataEnvelope, FireSolution, Paginated};
#[cfg(test)]
use grid::{fmt_grid, parse_grid, preview_pos};
#[cfg(test)]
use saved_fires::{
    hydration_step, restore, save_body, EventOption, SavedFire, SavedFor, Shown,
};
#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use weapon_selector::WEAPONS;

#[cfg(test)]
#[path = "tests/mortar.rs"]
mod tests;
