//! The operations schedule page and the card its master list repeats.
//!
//! **Role:** declares the route component and the operation card, and re-exports the page for
//! the router.
//! **Position:** the `/events` route, in the operations hub.
//! **Signals & state:** none at this level; the page owns the selection signal and both fetches.
//! **Invariants:** the detail column renders the same hub body the standalone operation route
//! does, so a briefing reads identically in both places.
#![allow(dead_code)]

mod page;
mod upcoming_ops;

pub use page::EventSchedulePage;
