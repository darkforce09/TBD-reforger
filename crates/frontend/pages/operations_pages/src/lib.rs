//! The operations hub: scheduling a live operation, briefing it, and slotting into it.
//!
//! **Role:** groups the pages that run the event lifecycle — the schedule, the operation
//! dossier with its inline slotting, the standalone slotting view, the caller's own service
//! record, and the global ladders.
//! **Position:** a page crate above the foundation crates (`frontend_session`,
//! `frontend_transport`, `frontend_api_dtos`, `frontend_ui`); the app's route table mounts its
//! pages at `/events`, `/events/:id`, `/events/:id/missions/:emid/orbat`, `/deployments` and
//! `/leaderboards`.
//! **Signals & state:** none at this level; every page owns its own fetches and signals.
//! **Invariants:** the schedule's detail column and the standalone operation route render the
//! same hub body, so a change to the dossier reaches both. The route components fetch in the
//! browser only, so they exist on `wasm32` alone; the pure readers under them compile on every
//! target for the native tests.

pub mod deployments;
pub mod event_detail;
pub mod leaderboards;
pub mod orbat_selection;
pub mod prelude;
pub mod schedule;
