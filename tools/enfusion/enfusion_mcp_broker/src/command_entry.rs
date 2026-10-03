//! `mcpd`'s entry: the choice between the broker and the stub.
//!
//! **Role:** reads `mcpd`'s arguments and environment, picks the stub or the broker, and turns
//! the broker's outcome into the process exit code.
//!
//! ```text
//!   mcpd --socket <path> [--pidfile <path>]
//!   mcpd --stub                                (or MCP_STUB=1 without --socket)
//! ```
//!
//! **Position:** the crate's entry, called by the `mcpd` binary of `developer_tools`.
//! **Signals & state:** reads the process arguments and `MCP_STUB`, `MCP_SOCK`; builds the tokio
//! runtime the broker runs on.
//! **Invariants:** broker mode wins whenever `--socket` is present; a missing socket is exit 2.

use std::path::PathBuf;
use std::process::ExitCode;

/// Runs `mcpd` with this process's arguments and returns its exit code.
pub fn run() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    // Broker mode wins whenever --socket is present: the selftest launches the broker with
    // MCP_STUB=1 exported (so the broker's CHILD — spawned argv-less from ENFUSION_MCP_BIN —
    // resolves to the stub), and the flag must not capture the broker itself.
    let has_socket = argv.iter().any(|a| a == "--socket");
    if !has_socket
        && (std::env::var("MCP_STUB").as_deref() == Ok("1")
            || argv.first().map(String::as_str) == Some("--stub"))
    {
        return crate::stub_server::run_stub();
    }
    let get = |k: &str| {
        argv.iter()
            .position(|a| a == k)
            .and_then(|i| argv.get(i + 1))
            .cloned()
    };
    let Some(sock) = get("--socket").or_else(|| std::env::var("MCP_SOCK").ok()) else {
        eprintln!("mcp-daemon: --socket required");
        return ExitCode::from(2);
    };
    let pidfile = get("--pidfile").unwrap_or_else(|| format!("{sock}.pid"));
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(crate::broker::run_broker(
        PathBuf::from(sock),
        PathBuf::from(pidfile),
    ))
}
