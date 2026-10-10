# Mod wave driver internals

The subcommands of `cargo xtask mod wave`, the [wave](/documentation/glossary/n_to_z.md#wave) driver
of the game-mod program: where the current wave stands, the mod wave gate, landing finished slices
and pushing `main`.

## Contents

```text
tools/commands/mod_operations/src/wave_execution/
├── execution.rs  dispatch, the lock and registry readers, `status`, `prep` and `gate`
└── land.rs       `land` (refuse dirty, merge, gate, reap, push) and `push`
```

## How it works

`tools/commands/mod_operations/src/wave_execution.rs` holds the help text and the worktree
states, and declares both files.

- The driver reads the shared wave plan from the central ticket manager (`ttm wave show`) and
  keeps only the children of the mod program ticket `MOD_PROGRAMME` (`T-181`, resolved with
  `ttm show`); a child's shipped state comes from that program's children. Both are read once per
  process. A plan or program the ticket manager cannot give is a refusal (exit 2), never "all
  shipped".
- The current wave is the first open wave (number above 0) with an unshipped mod slice. A slice's
  worktree is `.ai/artifacts/worktrees/<slice>/` on branch `slice/<slice>`; a sub-slice of three
  dot segments shares its two-segment parent's (`ticket_manager_client::parent_slice`).
- `prep`, `land` and the reap call `platform_execution::slice_worktree::run_at` in-process
  with `new`, `merge` and `reap`.
- `gate` runs the seven steps of `GATE_STEPS` in order and prints PASS or FAIL for each, with the last 12 output lines
  of a failure. It fails when any step fails:

  | Step | Command |
  |---|---|
  | compile | `cargo run -q -p xtask -- mod compile` |
  | world boot, world boot +mission | `mod world-boot` and `--mission=bridgehead-at-levie` |
  | schema validate | `cargo run -q -p xtask -- ci schema-validate` |
  | capability, oracle citations | `cargo run -q -p developer_tools --bin enf -- capability` and `-- citations` |
  | enf unit tests | `cargo test -q -p enfusion_script_index --lib`, through `distrobox-host-exec` |

- `push` counts the commits ahead of the upstream. It refuses when any changed path lies under
  `assets/terrains/`; otherwise it runs `git push --no-verify origin main`.

## Boundaries

- Depends on: `ticket_manager_client` (`show`, `wave_show`, `parent_slice`),
  `platform_execution::slice_worktree`, `repository_layout::WORKTREES_DIR`,
  `process_runner::Run`, and `git`.
- Used by: `tools/commands/mod_operations/src/wave_execution.rs`, which re-exports `run` to the
  `mod` dispatch.
- Rules: a missing wave plan refuses instead of reporting every wave shipped; `land` refuses a dirty
  worktree before merging anything; an unknown subcommand prints the help and exits 2; no gate
  step runs `make`, since the repository has no Makefile.

## Related documentation

- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the wave cycle these
  subcommands automate and the slice gate a merge needs.
