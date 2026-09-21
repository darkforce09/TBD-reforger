//! Ticket command services and platform-execution adapters.
pub use ticket_engine::cli::*;
mod execution;
pub use execution::{cmd_clean, cmd_done, cmd_run};

pub use ticket_engine::{registry::load_registry, sync::cmd_sync, validation::cmd_check};

pub use ticket_engine::metrics::cmd_metrics;

pub(crate) mod cli;
pub(crate) mod dispatch;
