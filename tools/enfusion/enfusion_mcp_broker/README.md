# Enfusion MCP broker

The `enfusion_mcp_broker` crate, the library behind the `mcpd` binary: a daemon that starts one
`enfusion-mcp` server, initialises it once (paying the server's index load a single time), and
serves tool calls from a Unix socket, so each `cargo xtask mcp call` costs one request instead of
a server start. The same binary is the offline stub `cargo xtask mcp selftest` runs in place of
the real server.

## Contents

```text
tools/enfusion/enfusion_mcp_broker/
├── Cargo.toml  the `enfusion_mcp_broker` library package: `enfusion_mcp`, `repository_layout`, `serde_json`, `thiserror`, `tokio`; layout tier 3
└── src/        the entry that picks broker or stub, the broker, the stub, the error type
```

## How it works

```text
mcpd --socket <path> [--pidfile <path>]          mcpd --stub   (or MCP_STUB=1 without --socket)
  │                                                │
  ├─ resolve the server (enfusion_mcp::server_entrypoint), spawn it, initialize (id 1)
  ├─ bind the socket, write the pidfile             └─ emulate the server's stdout:
  ├─ each connection: one {"tool","args"} line          one-shot (STUB_MODE, STUB_LINGER)
  │    ─▶ queued tools/call to the child                or request/response (STUB_DAEMON=1)
  │    ─▶ the answer, relabelled id 2, one line back
  └─ stop on SIGTERM, SIGINT, MCP_DAEMON_IDLE, MCP_DAEMON_MAX_LIFE, a socket failure or a failed
     restart: kill the child, remove the socket and the pidfile, return the exit code
```

A server child that exits fails its pending requests with a `child error` answer and is started
again; a failed restart stops the daemon with exit 1. `cargo xtask mcp daemon` builds and starts
this binary in its own session; `cargo xtask mcp daemon stop-all` matches it by `mcpd --socket`.
`src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo build -p developer_tools --bin mcpd   # the binary that calls this crate
cargo xtask mcp selftest                    # drives the stub and a stub-backed broker offline
```

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `ENFUSION_MCP_BIN` | the pinned npm package's module | the server to start; `cargo xtask mcp daemon` sets it to the entry it resolved |
| `MCP_SOCK` | none | the socket when `--socket` is absent |
| `MCP_DAEMON_IDLE` | 1800 s | stop after this long without a connection; 0 never |
| `MCP_DAEMON_MAX_LIFE` | 14400 s | stop after this long in any case; 0 never |
| `MCP_CALL_TIMEOUT` | 180 s | the limit on the initialisation and on each tool call |
| `MCP_DEBUG` | off | `1` logs the child's stderr and each request to stderr |
| `MCP_STUB` | off | `1` without `--socket` runs the stub |
| `STUB_MODE`, `STUB_DAEMON`, `STUB_LINGER` | `success`, off, 1 s | the stub's answers, request/response mode and one-shot linger |

## Boundaries

- Depends on: `enfusion_mcp` (`server_entrypoint`), `repository_root` (the root walk),
  `serde_json`, `thiserror`, `tokio` (runtime, Unix socket, child process, signals, timers).
- Used by: the `mcpd` binary in `tools/developer_tools/src/bin/mcpd.rs`; through it,
  `cargo xtask mcp daemon`, `cargo xtask mcp call` and `cargo xtask mcp selftest`.
- Rules: tier 3 of `tools/enfusion`; xtask never depends on this crate (no tokio in its closure,
  `cargo xtask verify crate-tiers`); the child is spawned through `tokio::process`, because the
  broker streams its stdio for the daemon's whole life, which the synchronous `process_runner`
  cannot host; only `run` decides the exit code.

## Related documentation

- [Enfusion MCP client](/tools/commands/enfusion_mcp/README.md) — the `cargo xtask mcp` commands
  that start and call this daemon.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the broker, the pinned
  server package and calling Workbench from the command line.
