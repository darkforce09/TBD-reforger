//! The announcement board and its two halves.
//!
//! **Role:** declares the route component, the master feed of dispatches, and the reading pane
//! that shows the selected one.
//! **Position:** the `/announcements` and `/announcements/:id` routes.
//! **Signals & state:** none at this level.
//! **Invariants:** the two halves read the same fetched payload; nothing here fetches twice.

mod article_feed;
mod article_viewer;
mod page;

pub use page::AnnouncementsPage;
