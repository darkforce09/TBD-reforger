//! The `cargo xtask mcp` command line.
//!
//! **Role:** the clap subcommands of `cargo xtask mcp`, each with its exit-code contract.
//! **Position:** the xtask binary nests [`McpCmd`] under its `mcp` command and hands the parsed
//! value to [`crate::run`].
//! **Signals & state:** none; plain data.
//! **Invariants:** `wb-logs` parses its own `--help` and an empty `--file` value, so neither
//! becomes a clap exit 2 that could read as a spawn verdict.

use clap::Subcommand;
use std::path::PathBuf;

/// One `cargo xtask mcp` command.
#[derive(Subcommand, Debug)]
pub enum McpCmd {
    /// Read NDJSON JSON-RPC from stdin; print id==2 result (exit 0/1/2/3)
    Consume,
    /// AF_UNIX client → daemon; print response line (exit 0/7)
    #[command(name = "socket-send")]
    SocketSend {
        /// The daemon's socket path.
        sock: String,
        /// The tool to call.
        tool: String,
        /// The tool's arguments as a JSON object.
        #[arg(default_value = "{}")]
        args_json: String,
    },
    /// Probe AF_UNIX socket connectability (exit 0/1)
    #[command(name = "probe-sock")]
    ProbeSock {
        /// The socket path to probe.
        sock: String,
    },
    /// Daemon-first JSON-RPC tool call.
    /// Exit: 0 success · 1 usage/empty · 2 init-failed · 3 tool error · 4 timeout.
    Call {
        /// The tool to call; absent or empty is usage (exit 1).
        tool: Option<String>,
        /// JSON object; defaults to `{}` when omitted or empty.
        args_json: Option<String>,
    },
    /// Offline MCP call-path selftest.
    /// Exit: 0 ALL PASS · 1 any arm failed.
    #[command(name = "selftest")]
    Selftest,
    /// Live wb_connect + wb_state smoke.
    /// Exit: 0 OK · 1 FAIL.
    Smoke,
    /// Raw Workbench NET API call (`<APIFunc> [json]`, e.g.
    /// `EMCP_WB_TbdBlueprint {"action":"recon","filter":"FarmHouse_E_1L01_Wood"}`).
    /// Exit: 0 ok (JSON on stdout) · 1 usage · 2 cannot connect · 3 Workbench error.
    Wbcall {
        /// The NET API function, such as `EMCP_WB_TbdBlueprint`; absent or empty is usage.
        api_func: Option<String>,
        /// The request's JSON object; absent or blank is `{}`.
        args_json: Option<String>,
        /// Read/connect timeout in seconds (recon on a 1.2 M-entity world needs minutes).
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
    /// setsid + AF_UNIX socket lifecycle.
    /// Exit: 0 success · 1 stopped/fail · 2 usage.
    Daemon {
        /// start|stop|status|restart|stop-all (default: status)
        action: Option<String>,
    },
    /// Grep latest Workbench Play console.log for TBD spawn diagnostics.
    /// Exit: 0 PASS · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT.
    #[command(name = "wb-logs", disable_help_flag = true)]
    WbLogs {
        /// Verdict over a specific log file (no Workbench).
        /// Bare `--file` / `--file ''` → usage rc=3; `--file=` → ENVIRONMENT rc=3.
        /// Custom parser accepts empty (PathBufValueParser would clap-exit 2).
        #[arg(
            long,
            num_args = 0..=1,
            default_missing_value = "__MISSING__",
            value_parser = crate::workbench_logs::parse_file_arg
        )]
        file: Option<PathBuf>,
        /// Prove the verdict logic can FAIL
        #[arg(long)]
        selftest: bool,
        /// Usage (exit 3, never a spawn verdict)
        #[arg(short = 'h', long = "help")]
        help: bool,
        /// Display extract pattern only (does not affect the verdict)
        pattern: Option<String>,
    },
}
