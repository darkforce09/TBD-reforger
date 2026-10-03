# Xtask fixtures

Recorded inputs that xtask commands replay to check themselves without the outside systems they
normally talk to.

## Contents

```text
tools/xtask/fixtures/
└── mcp/  recorded enfusion-mcp responses that `cargo xtask mcp selftest` replays
```

## Format

- Encoding: each child folder holds one kind of recording and states its own format; `mcp/`
  holds newline-delimited JSON-RPC 2.0.
- Schema: none in `contracts/`; the command that replays a folder defines what it expects.
- Adding a file: add it to the folder of its kind, with the check that reads it, and name the
  folder in `tools/foundation/repository_layout/src/tool_inputs.rs` when it is new.

## Producers and consumers

- Producers: people, from recorded tool output.
- Consumers: `cargo xtask mcp selftest`, for `mcp/`, through `MCP_TRANSCRIPT_FIXTURES_DIR` in
  `tools/foundation/repository_layout/src/tool_inputs.rs`.

## Boundaries

- Depends on: the commands whose output the recordings stand in for.
- Used by: `tools/commands/enfusion_mcp/src/call_selftest.rs`.
- Rules: a fixture folder is reached only through its constant in
  `tools/foundation/repository_layout/src/tool_inputs.rs`, and
  `every_committed_location_exists_in_the_checkout` in
  `tools/foundation/repository_layout/src/tests/command_locations_tests.rs` checks the folder exists; test fixtures of the other tooling crates live with
  those crates.
