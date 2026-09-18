use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum McpCmd {
    /// Read NDJSON JSON-RPC from stdin; print id==2 result (exit 0/1/2/3)
    Consume,
    /// AF_UNIX client → daemon; print response line (exit 0/7)
    #[command(name = "socket-send")]
    SocketSend {
        sock: String,
        tool: String,
        #[arg(default_value = "{}")]
        args_json: String,
    },
    /// Probe AF_UNIX socket connectability (exit 0/1)
    #[command(name = "probe-sock")]
    ProbeSock { sock: String },
    /// Daemon-first JSON-RPC tool call (T-860 port of mcp-call.sh).
    /// Exit: 0 success · 1 usage/empty · 2 init-failed · 3 tool error · 4 timeout.
    Call {
        tool: Option<String>,
        /// JSON object; defaults to `{}` when omitted or empty.
        args_json: Option<String>,
    },
    /// Offline MCP call-path selftest (T-865 port of mcp-call-selftest.sh).
    /// Exit: 0 ALL PASS · 1 any arm failed.
    #[command(name = "selftest")]
    Selftest,
    /// Live wb_connect + wb_state smoke (T-877 port of mcp-smoke.sh).
    /// Exit: 0 OK · 1 FAIL.
    Smoke,
    /// T-090.11.3 — raw Workbench NET API call (`<APIFunc> [json]`, e.g.
    /// `EMCP_WB_TbdBlueprint {"action":"recon","filter":"FarmHouse_E_1L01_Wood"}`).
    /// Exit: 0 ok (JSON on stdout) · 1 usage · 2 cannot connect · 3 Workbench error.
    Wbcall {
        api_func: Option<String>,
        args_json: Option<String>,
        /// Read/connect timeout in seconds (recon on a 1.2 M-entity world needs minutes).
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
    /// setsid + AF_UNIX socket lifecycle (T-888 port of mcp-daemon.sh).
    /// Exit: 0 success · 1 stopped/fail · 2 usage.
    Daemon {
        /// start|stop|status|restart|stop-all (default: status)
        action: Option<String>,
    },
    /// Grep latest Workbench Play console.log for TBD spawn diagnostics (T-857).
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
            value_parser = crate::commands::mcp::workbench_logs::parse_file_arg
        )]
        file: Option<PathBuf>,
        /// Prove the verdict logic can FAIL
        #[arg(long)]
        selftest: bool,
        /// Usage (exit 3 — matches former mcp-wb-logs.sh)
        #[arg(short = 'h', long = "help")]
        help: bool,
        /// Display extract pattern only (does not affect the verdict)
        pattern: Option<String>,
    },
}
