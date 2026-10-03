//! The Enfusion MCP client behind `cargo xtask mcp`.
//!
//! **Role:** parses and runs every `cargo xtask mcp` command ([`McpCmd`], [`run`]): a tool call
//! through the `mcpd` broker or one-shot ([`daemon`] starts, stops and probes the broker), the
//! JSON-RPC consumer and socket client the call path spawns, the offline selftest and the live
//! smoke, the raw Workbench NET API call, and the Workbench log verdict; [`server_entrypoint`]
//! resolves the command that starts an `enfusion-mcp` server for this crate and for the broker.
//! **Position:** tier 2 of `tools/commands`, over `process_runner`, `repository_layout` and
//! `verification_core`. The xtask binary parses `McpCmd` and calls [`run`]; xtask's mod
//! bootstrap starts the daemon through [`daemon`]; `enfusion_mcp_broker` (the `mcpd` binary)
//! resolves its server child through [`server_entrypoint`].
//! **Signals & state:** process-global environment only: `cargo xtask mcp call` exports its
//! Enfusion defaults and `MCP_SOCK` before it spawns a child, and the selftest exports the stub
//! selection; the daemon's state lives in its socket, pidfile and log beside the socket.
//! **Invariants:** no tokio and no broker dependency, so xtask's dependency closure stays free of
//! them; every command answers with its documented exit code, never a panic; the server command
//! is resolved in one place for the client and the broker.

mod call;
mod call_selftest;
mod command_line;
pub mod daemon;
mod dispatch;
mod error;
mod json_rpc;
mod netapi;
pub mod prelude;
pub mod server_entrypoint;
mod smoke;
mod workbench_logs;

pub use command_line::McpCmd;
pub use dispatch::run;
pub use error::{Error, Result};
pub use workbench_logs::preprocess_cli_args;
