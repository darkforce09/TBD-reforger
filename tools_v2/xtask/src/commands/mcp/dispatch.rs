use super::cli::McpCmd;
use anyhow::Result;

pub(crate) fn run(cmd: McpCmd) -> Result<u8> {
    {
        let code = match cmd {
            McpCmd::Consume => crate::commands::mcp::json_rpc::cmd_consume(),
            McpCmd::SocketSend {
                sock,
                tool,
                args_json,
            } => {
                if sock.is_empty() || tool.is_empty() {
                    eprintln!("usage: mcp-socket-send <socket> <tool> [args-json]");
                    7
                } else {
                    crate::commands::mcp::json_rpc::cmd_socket_send(&sock, &tool, &args_json)
                }
            }
            McpCmd::ProbeSock { sock } => crate::commands::mcp::json_rpc::cmd_probe_sock(&sock),
            McpCmd::Call { tool, args_json } => crate::commands::mcp::call::run(tool, args_json),
            McpCmd::Selftest => crate::commands::mcp::call_selftest::run(),
            McpCmd::Smoke => crate::commands::mcp::smoke::run(),
            McpCmd::Wbcall {
                api_func,
                args_json,
                timeout,
            } => crate::commands::mcp::netapi::cmd(
                api_func.as_deref(),
                args_json.as_deref(),
                timeout,
            ),
            McpCmd::Daemon { action } => crate::commands::mcp::daemon::cmd(action.as_deref()),
            McpCmd::WbLogs {
                file,
                selftest,
                help,
                pattern,
            } => {
                return crate::commands::mcp::workbench_logs::run(file, selftest, help, pattern);
            }
        };
        Ok(code as u8)
    }
}
