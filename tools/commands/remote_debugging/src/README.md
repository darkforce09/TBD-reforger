# Remote debugging source

The `cargo xtask debug` group with the `mod remote-logs` verdict, the `cargo xtask repro` group,
and the errors they report.

## Contents

```text
tools/commands/remote_debugging/src/
├── debug/         `debug`: the A2S, NDJSON and direct-join probes, the fleet instance selection and the `mod remote-logs` verdict
├── error.rs       `Error` and `Result`, and the crate-private context trait
├── lib.rs         the crate root: module header, `mod` lines and the re-exports
├── prelude.rs     `DebugCmd`, `ReproCmd`, `Error` and `Result` for glob import
└── reproduction/  `repro`: the mission-version upload reproduction and its two request helpers
```

## How it works

- Each group folder holds its clap enum (`debug_command.rs`, `reproduction_command.rs`), its
  dispatch (`debug_dispatch.rs`, `reproduction_dispatch.rs`, each with a `run`) and the modules
  that do the work; its `mod.rs` re-exports the enum and `run`.
- `error`: a refused input or a probe's outcome is an exit code the command returns; an `Error`
  is a failure the command cannot recover from, printed as `<step>: <cause>`, the text these
  commands have always printed.

## Boundaries

- Depends on: `deploy_settings`, `deployment`, `process_runner`, `repository_layout`,
  `verification_core`, `time_source`, `clap`, `serde_json`, `regex` and `thiserror`.
- Used by: the crate root's re-exports and public modules, read by the `debug`, `repro` and `mod`
  groups of `xtask`.
