//! The strict-check banner model, the `git status` chip and the debounced file watch.
//!
//! **Role:** turns `cargo xtask ticket check --strict` output and exit codes into a check model,
//! `git status` output into a dirty chip, and file-change notifications into debounced reloads.
//! **Position:** over [`crate::core`]; [`crate::application_state`] and the application drive the
//! subprocesses, and `tools/tickets/ticketboard_desktop`'s `repository_status::ui` paints
//! [`models::view::StatusView`] and emits [`events::StatusEvent`]s.
//! **Signals & state:** the file watch runs on the `notify` thread and reports over a channel;
//! the models are plain data.
//! **Invariants:** concurrent triggers coalesce into one follow-up run; nothing here names egui.

pub mod events;
pub mod models;
pub mod services;
