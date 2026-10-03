# Enfusion MCP client source

The modules of the `enfusion_mcp` crate: one per `cargo xtask mcp` command, the command line and
its dispatch, and the resolution of the `enfusion-mcp` server command the call path, the daemon
launcher and the broker share.

## Contents

```text
tools/commands/enfusion_mcp/src/
├── call.rs               `call`: daemon first, then a one-shot server with timeout and retries
├── call_selftest.rs      `selftest`: recorded transcripts and the `mcpd` stub, offline
├── command_line.rs       the `McpCmd` clap enum: nine commands
├── daemon.rs             `daemon`: builds and starts `mcpd`, status, stop, restart and stop-all
├── dispatch.rs           `run`: routes each `McpCmd`; `socket-send` refuses an empty socket or tool
├── error.rs              `Error` and `Result`: why a Workbench NET API call gave no answer
├── json_rpc.rs           `consume`, `socket-send` and `probe-sock`: the JSON-RPC and socket pieces
├── lib.rs                the crate root: module header, `mod` lines and the re-exports
├── netapi.rs             `wbcall`: the Workbench NET API wire protocol over TCP
├── prelude.rs            `McpCmd`, `run`, `preprocess_cli_args` and the server command types for glob import
├── server_entrypoint.rs  which `enfusion-mcp` server command a caller starts, in four tiers, and its process pattern
├── smoke.rs              `smoke`: live `wb_connect` and `wb_state` calls through `call`
├── tests/                unit tests for call, the self-test, daemon, netapi, the server entrypoint, smoke and wb-logs
└── workbench_logs.rs     `wb-logs`: the spawn verdict over the newest Workbench Play console log
```

## How it works

```text
xtask: preprocess_cli_args ─▶ clap ─▶ McpCmd ─▶ run (dispatch.rs)
   call ──────────▶ daemon::start_at / is_running_at ─▶ `mcp socket-send` ─▶ `mcp consume`
        └─ else ──▶ server_entrypoint::resolve ─▶ `timeout <server>` ─▶ `mcp consume`
   daemon ────────▶ cargo build -p developer_tools --bin mcpd ─▶ detached `mcpd --socket`
   selftest/smoke ▶ cargo run -q -p xtask -- mcp … (children through process_runner)
   wbcall ────────▶ netapi: one TCP connection to the Workbench NET API
   wb-logs ───────▶ workbench_logs: the verdict over one console log
```

- Every child goes through `process_runner`; the `mcpd` broker, which must outlive the command
  that started it, is its detached spawn, in a new session with its output in the log.
- `call` and the selftest write the process environment (`std::env::set_var`) only before they
  spawn a child, and `getuid` names the socket; those calls carry reasoned `#[allow(unsafe_code)]`.
- `error.rs` holds the NET API failures; each prints its cause after `: `, the line `wbcall`
  prints before it maps a connection failure to 2 and any other to 3.

## Boundaries

- Depends on: `process_runner`, `repository_layout`, `verification_core`, `clap`, `libc`,
  `serde_json`, `thiserror`; `tool_test_support` in `tests/`.
- Used by: the xtask binary (`McpCmd`, `run`, `preprocess_cli_args`), xtask's mod development
  bootstrap (`daemon`), and `enfusion_mcp_broker` (`server_entrypoint`).
- Rules: the exit codes in the [crate README](/tools/commands/enfusion_mcp/README.md) are pinned
  by `cargo xtask mcp selftest`; the server command is resolved only in `server_entrypoint.rs`.
