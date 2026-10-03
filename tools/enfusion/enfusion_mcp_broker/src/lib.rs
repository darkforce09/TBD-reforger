//! The `mcpd` broker over one `enfusion-mcp` server, and its offline stub.
//!
//! **Role:** [`run`] reads `mcpd`'s arguments and environment and either serves tool calls over
//! a Unix socket through one long-lived, initialised server child (the broker), or emulates the
//! server's newline-JSON stdout without a Workbench (the stub).
//! **Position:** tier 3 of `tools/enfusion`, over `enfusion_mcp` (which server command to start)
//! and `repository_layout` (the checkout it resolves against). The `mcpd` binary of
//! `developer_tools` calls [`run`]; `cargo xtask mcp daemon` starts that binary, and
//! `cargo xtask mcp selftest` runs it as the stub.
//! **Signals & state:** the broker owns a tokio runtime, the server child and its pipes, the
//! table of pending requests, the call queue and the idle clock for the daemon's life; the stub
//! holds no state.
//! **Invariants:** requests reach the server one at a time; every answer goes back relabelled to
//! `id == 2`; the daemon stops on SIGTERM, SIGINT, the idle limit or the lifetime limit, removing
//! its socket and pidfile; the exit code is decided by [`run`], never by a task.

mod broker;
mod command_entry;
mod error;
pub mod prelude;
mod stub_server;

pub use command_entry::run;
pub use error::{Error, Result};
