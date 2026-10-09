//! The ticketboard's headless half: everything the desktop viewer knows, without painting it.
//!
//! **Role:** loads the `.ai/tickets/` corpus, the wave lock, the run receipts, the estimates and
//! the scope vocabulary ([`application_state::background_loading`]), projects them into the board,
//! tree, filter, facet, wave-lane and metrics models, reads repository documents, queues and
//! guards `cargo xtask ticket` commands, models the strict check, `git status` and the file watch,
//! streams subprocess output ([`core::process`]), and holds the egui-free application state
//! ([`application_state::workspace_state::WorkspaceState`], [`application_state::events::Action`]).
//! **Position:** tier 4 of `tools/tickets`, over `ticket_model`, `ticket_metrics`,
//! `ticket_wave_lock`, `repository_layout` and `time_source`; `tools/tickets/ticketboard_desktop`
//! paints these models with egui and applies the actions its views emit.
//! **Signals & state:** the models are plain data; the loaders, the document reads, the file
//! watch and the subprocess streams run on worker threads and report over `mpsc` channels.
//! **Invariants:** nothing here names egui or eframe; the crate writes no file under the
//! repository: every ticket change is a `cargo xtask ticket` subprocess; a malformed ticket
//! refuses the whole corpus, while wave-lock, receipt, estimate and vocabulary failures stay
//! local to their displays.

pub mod application_state;
pub mod core;
pub mod document_viewer;
mod error;
pub mod execution_metrics;
pub mod prelude;
pub mod repository_status;
pub mod ticket_actions;
pub mod ticket_browser;
pub mod ticket_registry;
pub mod wave_plan;

pub use error::{Error, Result};
