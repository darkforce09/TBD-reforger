# Enfusion MCP broker source

The modules of the `enfusion_mcp_broker` crate: the entry that reads `mcpd`'s arguments, the
broker over one server child, the offline stub, and the error of a failed child start.

## Contents

```text
tools/enfusion/enfusion_mcp_broker/src/
├── broker.rs         the daemon: server child, initialisation, serialised calls, restart, idle and lifetime limits, stop requests
├── command_entry.rs  `run`: broker or stub from the arguments and `MCP_STUB`; the tokio runtime
├── error.rs          `Error` and `Result`: why the server child could not be started
├── lib.rs            the crate root: module header, `mod` lines and the re-exports
├── prelude.rs        `run` for glob import
└── stub_server.rs    the offline stub: one-shot or request/response answers on stdio
```

## How it works

- `broker.rs`: the signal, idle and lifetime tasks and a failed restart send an exit code over a
  channel; the accept loop alone stops the daemon (kill the child, remove the socket and the
  pidfile) and returns the code to `command_entry::run`, which returns it to the binary. A stop
  request during the first initialisation ends the daemon at once.
- `stub_server.rs`: a closed stdout (a consumer that stopped reading) ends the stub with exit 0.

## Boundaries

- Depends on: `enfusion_mcp`, `repository_layout`, `serde_json`, `thiserror`, `tokio`.
- Used by: `tools/developer_tools/src/bin/mcpd.rs`.
- Rules: no `std::process::exit`; the child is a `tokio::process` child, with the reason beside
  the spawn.
