//! The settings page: the viewer's profile, game-account link and service record.
//!
//! **Role:** declares the route component and re-exports it for the router.
//! **Position:** the `/settings` route, in the account group.
//! **Signals & state:** none at this level; the page owns both fetches and every signal.
//! **Invariants:** the page is single-file — the three cards are one settled render over two
//! resources, and splitting them would mean fetching or gating twice.

mod page;

pub use page::SettingsPage;
