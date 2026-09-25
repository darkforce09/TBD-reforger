# Xtask source

The source of the `xtask` binary behind `cargo xtask`: the command tree, the command groups, the
repository verifications they run, and the shared plumbing they stand on.

## Contents

```text
tools_v2/xtask/src/
├── cli/            the top-level clap tree and the dispatch to each command group
├── commands/       one folder per command group: its clap enum, dispatch and work
├── core/           repository root and layout, cargo target directory, host execution, test setup
├── main.rs         the binary entry: runs the dispatch and turns its result into the exit code
├── tests/          crate-level tests: dependency direction, file limits, prose rules, layout paths
└── verifications/  the repository checks behind `cargo xtask verify`, grouped by invariant
```

## How it works

```text
main.rs ──▶ cli::dispatch::run ──▶ commands::<group>::dispatch::run ──▶ the group's work
                                          │                               │
                                          ├──▶ verifications::<group>     ├──▶ core (root, layout,
                                          │    (verify, ci, platform)     │    target dir, host)
                                          └──▶ ticket-engine, developer-tools, verification-core
```

`main.rs` declares the modules, calls `cli::dispatch::run` and exits with the `u8` it returns, or
prints `xtask: <error chain>` and exits 1 on an error. Every command finds the checkout from the
working directory by walking up to `.ai/tickets/ROOT` (`find_repo_root` in `core/`), so a command
run inside a linked worktree reads that worktree's files, and joins the repository paths it needs
from the constants in `core/repository_layout.rs`. `commands/` holds the operational commands and
`verifications/` the checks; `cargo xtask verify`, the `ci` task table and the platform wave gate
call the same verification functions.

`main.rs` also wires the two crate-wide test files in `tests/`,
`tooling_dependency_boundaries.rs` and `tooling_prose_rules.rs`; the other files there are
wired by the modules they test.

## Public surface

- The `xtask` binary; the crate exposes no library. Everything else is crate-internal, reached
  through the command line described in `tools_v2/xtask/src/cli/README.md`.

## Boundaries

- Depends on: `ticket-engine` (ticket storage, the wave lock, repository paths and root
  discovery), `developer-tools` (engine-backed map and blueprint work) and `verification-core`
  (verdicts, process runs, scans), each by path; clap, serde and the other crates in
  `tools_v2/xtask/Cargo.toml`.
- Used by: `tools_v2/xtask/Cargo.toml`, whose one `[[bin]]` is `src/main.rs`.
- Rules:
  - xtask never depends on `website-map-engine` or `website-graphics-engine`, and
    `developer-tools` never depends on xtask (`tooling_dependency_direction_is_enforced`);
  - a production file stays under 500 lines, `main.rs` under 150 and a separate test file under
    1000 (`tooling_source_files_stay_below_their_structural_limits`), and no test module is inline
    (`tooling_test_modules_live_in_separate_files`);
  - comments, help text and documents under `tools_v2/` carry no ticket ids, retired spellings,
    script file names, history words or names of Rust files that do not exist
    (`tools_v2/xtask/src/tests/tooling_prose_rules.rs`).
