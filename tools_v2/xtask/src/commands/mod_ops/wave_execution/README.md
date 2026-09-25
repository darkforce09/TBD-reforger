# Mod wave driver internals

The subcommands of `cargo xtask mod wave`, the [wave](/documentation_v2/glossary.md#wave) driver
of the game-mod program: where the current wave stands, the mod wave gate, landing finished slices
and pushing `main`.

## Contents

```text
tools_v2/xtask/src/commands/mod_ops/wave_execution/
├── execution.rs  dispatch, the lock and registry readers, `status`, `prep` and `gate`
└── land.rs       `land` (refuse dirty, merge, gate, reap, push) and `push`
```

## How it works

`tools_v2/xtask/src/commands/mod_ops/wave_execution.rs` holds the help text and the worktree
states, and declares both files.

- The driver reads the shared wave lock through `ticket_engine::wave_lock` and keeps only the ids
  that are dotted children of the mod program ticket, which it reads as
  `game_mod_programme_ticket` from the corpus pins (`ticket_engine::corpus_pins`). Shipped slices
  come from that program ticket's `slice_plan`. A missing lock or pin file is a refusal (exit 2),
  never "all shipped".
- The current wave is the first open wave (number above 0) with an unshipped mod slice. A slice's
  worktree is `.ai/artifacts/worktrees/<slice>/` on branch `slice/<slice>`; a two-dot sub-slice
  shares its parent's.
- `prep`, `land` and the reap call `crate::commands::platform::slice_worktree::run_at` in-process
  with `new`, `merge` and `reap`.
- `gate` runs twelve steps in order and prints PASS or FAIL for each, with the last 12 output lines
  of a failure. It fails when any step fails:

  | Step | Command |
  |---|---|
  | compile, compile-selftest | `cargo run -q -p xtask -- mod compile` and `mod compile-selftest` |
  | world boot, world-boot selftest, world boot +mission | `mod world-boot`, `--selftest`, `--mission=bridgehead-at-levie` |
  | ui layouts | `cargo run -q -p xtask -- verify ui-layouts` |
  | schema validate, capability, oracle citations, no-crf-leak | `distrobox-host-exec make <target>` |
  | ticket registry | `ticket check`, through `distrobox-host-exec` |
  | enf unit tests | `cargo test -q -p developer-tools --lib enf::`, through `distrobox-host-exec` |

- `push` counts the commits ahead of the upstream. It refuses when any changed path lies under
  `assets_v2/terrains/`; otherwise it runs `git push --no-verify origin main`.

## Boundaries

- Depends on: `ticket_engine` (`corpus_pins`, `wave_lock`, `registry`),
  `crate::commands::platform::slice_worktree`, `crate::core::repository_layout::WORKTREES_DIR`,
  `verification_core::proc::Run`, and `git`.
- Used by: `tools_v2/xtask/src/commands/mod_ops/wave_execution.rs`, which re-exports `run` to the
  `mod` dispatch.
- Rules: a missing lock refuses instead of reporting every wave shipped
  (`missing_lock_is_a_refusal_not_all_shipped`); `land` refuses a dirty worktree before merging
  anything (`land_refuses_dirty_worktree`); an unknown subcommand prints the help and exits 2
  (`unknown_command_prints_help_rc2`); all in
  `tools_v2/xtask/src/commands/mod_ops/tests/wave_execution.rs`.
