//! The `mcpd` binary: the persistent enfusion-mcp broker daemon and its offline stub.

fn main() -> std::process::ExitCode {
    enfusion_mcp_broker::run()
}
