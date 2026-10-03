# Xtask source

The source of the `xtask` binary behind `cargo xtask`: the command tree and the command groups.
The repository verifications they run are the check and command crates under `tools/`. The shared plumbing they stand on (the checkout root and
layout, the deploy settings, child processes and the test locks) lives in the `tools/foundation`
crates.

## Contents

```text
tools/xtask/src/
├── cli/            the top-level clap tree and the dispatch to each command group
├── commands/       one folder per command group: its clap enum, dispatch and work
└── main.rs         the binary entry: runs the dispatch and turns its result into the exit code
```

## How it works

```text
main.rs ──▶ cli::dispatch::run ──▶ commands::<group>::dispatch::run ──▶ the group's work
                                          │                               │
                                          ├──▶ check and command crates   ├──▶ tools/foundation (root,
                                          │    (verify, ci, mk, deploy …) │    layout, settings, host)
                                          └──▶ ticket crates, map asset crates, tools/foundation crates
```

`main.rs` declares the modules, calls `cli::dispatch::run` and exits with the `u8` it returns, or
prints `xtask: <error chain>` and exits 1 on an error. Every command finds the checkout from the
working directory by walking up to `.ai/tickets/ROOT` (`find_repository_root` of the
`repository_layout` crate), so a command run inside a linked worktree reads that worktree's files,
and joins the repository paths it needs from that crate's constants. `commands/` holds the
command groups that still live in the binary; `cargo xtask verify`, the `ci` task table of
`ci_task_catalog` and the platform wave gate call the same verification functions of the check
and command crates.

The tooling rules that judge this crate with every other tool crate (dependency direction, line
limits, test placement, prose) are tests of the `repository_checks` crate
(`tools/checks/repository_checks/src/tests/`).

## Public surface

- The `xtask` binary; the crate exposes no library. Everything else is crate-internal, reached
  through the command line described in `tools/xtask/src/cli/README.md`.

## Boundaries

- Depends on: the ticket crates `ticket_model` (the ticket model and store),
  `ticket_metrics` (run receipts), `ticket_wave_lock` (the wave lock) and `ticket_registry`
  (ticket operations, checks and sync), `blueprint_compiler`, `map_asset_verification` and
  `world_export_pipeline` (the map commands and the map asset gates), `verification_core`
  (verdicts, scans), `process_runner` (process runs, the host bridge, the ssh transport, the
  `PATH` guard), `platform_execution` (the platform factory), `repository_layout` (the checkout root
  and the shared locations) and `deploy_settings` (the deploy settings file), and in its tests
  `tool_test_support` (the environment and working-directory locks, the test checkout root); clap, serde and the other crates in
  `tools/xtask/Cargo.toml`.
- Used by: `tools/xtask/Cargo.toml`, whose one `[[bin]]` is `src/main.rs`.
- Rules (tests in `tools/checks/repository_checks/src/tests/`):
  - xtask and `developer_tools` are binary-only packages whose workspace dependencies are tool
    crates; neither depends on the other or on a member under `legacy/`, and no member depends on
    either (`tooling_dependency_direction_is_enforced`);
  - no tokio, axum, reqwest, resvg or image enters xtask's dependency closure (rule 6 of
    `cargo xtask verify crate-tiers`);
  - a production file stays under 500 lines, `main.rs` under 150 and a separate test file under
    1000 (`tooling_source_files_stay_below_their_structural_limits`), and no test module is inline
    (`tooling_test_modules_live_in_separate_files`);
  - comments, help text and documents under `tools/` carry no ticket ids, retired spellings,
    script file names, history words or names of Rust files that do not exist, and no production
    file names a private-network address (`tools/checks/repository_checks/src/tests/tooling_prose_rules.rs`).
