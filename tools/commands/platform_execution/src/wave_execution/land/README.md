# Platform wave landing and close

The irreversible half of `cargo xtask platform wave`: `land` merges ready slices into `main`,
`revert` rolls `main` back, `verified` records the adversarial verifier's sha, and `wave --close`
writes the [wave](/documentation/glossary/n_to_z.md#wave)-close marker commit.

## Contents

```text
tools/commands/platform_execution/src/wave_execution/land/
├── close_ceremony.rs   the marker commit built unreachable, checked, then fast-forwarded; the close record
├── merge_execution.rs  `land`, `revert` and `verified`, with the landing records and the repack
└── wave_close.rs       `wave --close` arguments, close target, validations and the marker subject
```

## How it works

`tools/commands/platform_execution/src/wave_execution/land.rs` re-exports the four commands.

```text
land [--wave] [--bookkeeping] [<ticket>…]
  ├─ unknown argument, or a named ticket (slug or legacy number) not in the current wave ── exit 2
  ├─ ready = unshipped, worktree committed and clean, branch ahead of main
  │    (--wave holds every ready slice while any in the wave is unfinished)
  ├─ refuse a slice with no run receipt in the ticket manager, unless --bookkeeping
  ├─ refuse any slice whose gate verdict is missing, red or for another sha
  ├─ git merge --no-ff each, then `ttm land <slice> --sha <HEAD> [--require-receipt]`
  │    (stamps the newest receipt landed); stop at a conflict or a refused record
  ├─ full wave gate on merged main ── red: keep every worktree, print `revert <base>`, exit 1
  ├─ slice-worktree drop for each landed slice
  ├─ `ttm wave repack` (the plan lives in the ticket manager; nothing is committed)
  └─ push
```

- `revert <sha>` reverts every commit after the given green sha, parent 1 for merges, and leaves
  the slice branches in place; it then lists the tickets whose recorded landing commit was
  reverted, each with the `ttm unland <slice>` that clears the record.
- `verified <sha>` writes the full sha to `.ai/artifacts/last-verified`.
- `wave --close [--summary <text>] [--tickets <ids>] [--dry-run]` targets the oldest pending-close
  wave of the ticket manager's plan. It refuses unless every ticket of that entry is shipped, a verifier is
  recorded at or after the last landing, and the full wave gate passes on `main`.
  `close_ceremony` then builds the marker commit as an unreachable object, runs the marker
  checks of `super::base` on it, and fast-forwards `main` to it only if they accept. It then
  records the close with `ttm wave close <n> --sha <marker> [--members …]` (which repacks the
  plan), runs `ttm wave check`, and checks that the plan's wave base and the newest marker in git
  agree. When a step after the marker fails it says the marker is committed and that re-running
  the printed `ttm wave close` is safe. `--dry-run` prints the subject and writes nothing.

## Boundaries

- Depends on: `super::gate` (`cmd_gate`), `super::verdict` (`land_refusal`), `super::base`
  (marker checks), `super::ledger`, `super::push`, `crate::slice_worktree`
  (`drop`), `ticket_manager_client` (`land`, `list`, `wave repack`, `wave close`, `wave check`),
  and `git`.
- Used by: `tools/commands/platform_execution/src/wave_execution/flush.rs` (the `land`, `revert`,
  `verified` and `wave --close` dispatch).
- Rules: every argument parser is an allowlist, since a filter that is ignored lands more than was
  asked for; a red gate after merge never drops a worktree; only `close_ceremony` writes marker
  commits, and what it checked is what lands, by sha.

## Related documentation

- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — landing, shipping, the verifier record
  and the close, in order.
