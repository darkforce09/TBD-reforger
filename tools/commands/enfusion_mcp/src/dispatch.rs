//! Runs one parsed `cargo xtask mcp` command.
//!
//! **Role:** maps each [`McpCmd`] variant onto the module that runs it and returns that
//! module's exit code.
//! **Position:** the crate's entry; the xtask binary parses the command line into [`McpCmd`] and
//! calls [`run`].
//! **Signals & state:** none; every arm hands its arguments to the module that owns the command.
//! **Invariants:** an empty socket or tool for `socket-send` is usage (7), checked before any
//! connection; every other code is the command's own.

use crate::command_line::McpCmd;

/// Runs `cmd` and returns its exit code.
pub fn run(cmd: McpCmd) -> u8 {
    let code = match cmd {
        McpCmd::Consume => crate::json_rpc::cmd_consume(),
        McpCmd::SocketSend {
            sock,
            tool,
            args_json,
        } => {
            if sock.is_empty() || tool.is_empty() {
                eprintln!("usage: mcp-socket-send <socket> <tool> [args-json]");
                7
            } else {
                crate::json_rpc::cmd_socket_send(&sock, &tool, &args_json)
            }
        }
        McpCmd::ProbeSock { sock } => crate::json_rpc::cmd_probe_sock(&sock),
        McpCmd::Call { tool, args_json } => crate::call::run(tool, args_json),
        McpCmd::Selftest => crate::call_selftest::run(),
        McpCmd::Smoke => crate::smoke::run(),
        McpCmd::Wbcall {
            api_func,
            args_json,
            timeout,
        } => crate::netapi::cmd(api_func.as_deref(), args_json.as_deref(), timeout),
        McpCmd::Daemon { action } => crate::daemon::cmd(action.as_deref()),
        McpCmd::WbLogs {
            file,
            selftest,
            help,
            pattern,
        } => {
            return crate::workbench_logs::run(file, selftest, help, pattern);
        }
    };
    code as u8
}
