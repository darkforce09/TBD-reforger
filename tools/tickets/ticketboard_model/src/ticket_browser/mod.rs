//! The ticket browser's data: board columns and cards, the program tree, detail sections,
//! per-ticket filter facts, composable filters and the scope facets.
//!
//! **Role:** builds the projections the board, tree and detail views read, once per load, and
//! the filter verdicts and facet options, once per filter change.
//! **Position:** over [`crate::ticket_registry`]; [`crate::application_state`] owns the built
//! models, and `tools/tickets/ticketboard_desktop`'s `ticket_browser::ui` paints the borrowed
//! [`models::view::BrowserView`] and emits [`events::BrowserEvent`]s.
//! **Signals & state:** none; plain data rebuilt on load or filter change.
//! **Invariants:** filters never change the registry; nothing here names egui.

pub mod events;
pub mod models;
pub mod services;
