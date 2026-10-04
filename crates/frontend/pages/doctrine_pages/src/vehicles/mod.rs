//! The vehicle index: the faction-grouped list, the dossier pane, the administrator's vehicle form
//! and delete confirmation, and the route that binds them.
//!
//! **Role:** declares the route component, the master list, the dossier, the two dialogs and the
//! pure rules behind them — the form's validation, the write requests, the refusal wording and the
//! list updates — and re-exports the page for the router.
//! **Position:** the `/vehicles` route, in the doctrine hub.
//! **Signals & state:** none at this level; the page owns the fetch, the rows and every shared
//! signal.
//! **Invariants:** every pane reads the one fetched list, and an accepted write changes that list
//! in place — nothing here fetches a second time.

mod delete_confirmation;
pub mod page;
mod spec_drawer;
mod vehicle_draft;
mod vehicle_form_dialog;
mod vehicle_grid;
mod vehicle_rows;
mod vehicle_writes;
mod write_refusal;

#[cfg(target_arch = "wasm32")]
pub use page::VehicleDatabasePage;
