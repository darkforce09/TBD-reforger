# Slice worktree lifecycle internals

The subcommands of `cargo xtask platform slice-worktree`: one git worktree per slice under
`.ai/artifacts/worktrees/<slice>/`, on branch `slice/<slice>`, created from `main`, merged back,
and deleted only when nothing in it can be lost.

## Contents

```text
tools_v2/xtask/src/commands/platform/slice_worktree/
├── drop.rs       `drop` and `reap`, the two commands that delete worktrees, with their guards
├── git_plain.rs  git runners, root resolution, dispatch, `new` with its oracle lanes, `list`, `merge`
└── tests.rs      scenario tests in throwaway repositories for every subcommand and refusal
```

## How it works

`tools_v2/xtask/src/commands/platform/slice_worktree.rs` holds the usage text and declares the
three files. `run` resolves the root (`TBD_SLICE_WORKTREE_ROOT`, else the repository root) and
`run_at` takes an explicit one; both dispatch on the first argument. A missing slice id exits 2,
and an unknown subcommand prints the usage and exits 2. A two-dot sub-slice id maps to its
one-dot parent, so a sub-slice shares its parent's tree.

- `new <slice>`: adds the worktree from `main`, with git-lfs filters and hooks neutralised. It then
  links the gitignored oracle lanes `apps/mod/crf_framework` and `apps/mod/vanilla_reference`
  (required) and the PlayableSelector design mirror from `TBD_PS_ORACLE` (optional) into it.
  A missing required lane refuses with exit 1. Re-running `new` repairs a missing link.
- `list`: `git worktree list`.
- `merge <slice>`: refuses a missing, dirty or unreadable tree. It also refuses (exit 2) a slice
  whose gate verdict receipt is missing, red, or stamped for another sha
  (`crate::commands::platform::wave_execution::verdict::land_refusal`). It then runs
  `git merge --no-ff slice/<slice>` into the current branch.
- `drop <slice> [--force]`: refuses a branch with commits not on `main` and a dirty or unreadable
  tree unless `--force` is the third argument. It then force-removes the worktree, deletes the
  branch with `git branch -D` and prunes.
- `reap`: for every folder under the worktree base, keeps a tree that has no branch, is dirty or
  unreadable, is unstarted (no commits and no merge in `main`) or is not merged. It removes the
  rest without `--force` and deletes their branches with `git branch -d`.

## Boundaries

- Depends on: `verification_core::proc::Run`, `crate::core::repository_layout::WORKTREES_DIR`,
  `crate::commands::platform::wave_execution::verdict`, and `git`.
- Used by: `tools_v2/xtask/src/commands/platform/slice_worktree.rs` (`run`, `run_at`); in-process
  by `cargo xtask mod wave` (`prep`, `land`, the reap) and `cargo xtask platform wave land` (the
  drop after a green wave gate).
- Rules: output is a contract that `mod wave` reads; no guard of `drop` or `reap` is removed, since
  each covers a separate way to lose uncommitted work
  (`drop_refuses_a_dirty_tree_even_when_nothing_is_unmerged`,
  `reap_guards_every_destructive_case_in_one_pass`); `merge` refuses a slice no gate examined
  (`merge_refuses_a_slice_no_gate_has_examined`); `new` refuses a missing required oracle lane
  (`new_refuses_when_a_required_oracle_is_missing`); all in `tests.rs`.

## Related documentation

- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the slice lifecycle
  these commands automate.
- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — the same
  worktrees in the platform factory, and the `slice/<id>` branch exception to Law 2.
