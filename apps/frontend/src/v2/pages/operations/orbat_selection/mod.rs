//! The standalone slotting page.
//!
//! **Role:** declares the route component and re-exports it for the router.
//! **Position:** the `/events/:id/missions/:emid/orbat` route, in the operations hub.
//! **Signals & state:** none at this level; the page owns the fetch.
//! **Invariants:** the roster itself is the operation dossier's slotting selector, mounted here
//! on its own page rather than reimplemented.
#![allow(dead_code)]

mod page;

pub use page::OrbatSelectionPage;
