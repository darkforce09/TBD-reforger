//! The vehicle index: the faction-grouped list, the dossier pane and the route that binds them.
//!
//! **Role:** declares the route component, the master list and the detail dossier, and
//! re-exports the page for the router.
//! **Position:** the `/vehicles` route, in the doctrine hub.
//! **Signals & state:** none at this level; the page owns the fetch and both shared signals.
//! **Invariants:** both panes read the one fetched list — nothing here fetches a second time.
#![allow(dead_code)]

mod helpers;
mod page;
mod spec_drawer;
mod vehicle_grid;

pub use page::VehicleDatabasePage;
