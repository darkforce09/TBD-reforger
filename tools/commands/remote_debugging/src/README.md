# Remote debugging source

The `cargo xtask debug` group with the `mod remote-logs` verdict, and the errors they report.

## Contents

```text
tools/commands/remote_debugging/src/
├── debug/         `debug`: the A2S, NDJSON and direct-join probes, the fleet instance selection and the `mod remote-logs` verdict
├── error.rs       `Error` and `Result`, and the crate-private context trait
├── lib.rs         the crate root: module header, `mod` lines and the re-exports
└── prelude.rs     `DebugCmd`, `Error` and `Result` for glob import
```

## How it works

- `debug/` holds its clap enum (`debug_command.rs`), its dispatch (`debug_dispatch.rs`, with a
  `run`) and the modules that do the work; its `mod.rs` re-exports the enum and `run`.
- `error`: a refused input or a probe's outcome is an exit code the command returns; an `Error`
  is a failure the command cannot recover from, printed as `<step>: <cause>`, the text these
  commands have always printed.

## Boundaries

- Depends on: `deploy_settings`, `deployment`, `process_runner`, `repository_layout`,
  `verification_core`, `time_source`, `clap`, `serde_json`, `regex` and `thiserror`.
- Used by: the crate root's re-exports and public modules, read by the `debug` and `mod`
  groups of `xtask`.
