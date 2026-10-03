//! The `cargo xtask ticket` group: its clap tree, its dispatch onto the ticket crates, and the
//! side effects those crates leave out (running an agent, deleting worktrees and branches).
mod execution;

/// The registry projection loader, for the bin's `registry-get` verb.
pub use ticket_registry::load_registry;

pub(crate) mod cli;
pub(crate) mod dispatch;
