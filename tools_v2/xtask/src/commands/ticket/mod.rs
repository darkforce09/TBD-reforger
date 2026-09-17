//! Ticket command services and platform-execution adapters.
pub use ticket_engine::cli::*;
mod execution;
pub use execution::{cmd_clean, cmd_done, cmd_run};

pub use ticket_engine::{registry::load_registry, sync::cmd_sync, validation::cmd_check};

pub use ticket_engine::maintenance::{
    body_quarantine::cmd_quarantine_walls,
    main_goal_migration::cmd_migrate_main_goal,
    scope_migration::{cmd_migrate_v2, cmd_scope_histogram},
    timestamp_backfill::cmd_backfill_stamps,
};
pub use ticket_engine::metrics::cmd_metrics;
pub use ticket_engine::metrics::estimates::cmd_estimate_tokens;
