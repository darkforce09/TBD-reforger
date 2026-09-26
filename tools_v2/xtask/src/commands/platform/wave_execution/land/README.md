# Platform wave landing and close

The irreversible half of `cargo xtask platform wave`: `land` merges ready slices into `main`,
`revert` rolls `main` back, `verified` records the adversarial verifier's sha, and `wave --close`
writes the [wave](/documentation_v2/glossary/n_to_z.md#wave)-close marker commit.

## Contents

```text
tools_v2/xtask/src/commands/platform/wave_execution/land/
├── close_ceremony.rs   the marker commit built unreachable, checked, then fast-forwarded; lock repack
├── merge_execution.rs  `land`, `revert` and `verified`, with the receipt and lock commits after landing
└── wave_close.rs       `wave --close` arguments, close target, validations and the marker subject
```

## How it works

`tools_v2/xtask/src/commands/platform/wave_execution/land.rs` re-exports the four commands.

```text
land [--wave] [--bookkeeping] [<ticket id>…]
  ├─ unknown argument, or a named ticket not in the current wave ─────────── exit 2
  ├─ ready = unshipped, worktree committed and clean, branch ahead of main
  │    (--wave holds every ready slice while any in the wave is unfinished)
  ├─ refuse without slice-run receipts (ticket_engine::metrics), unless --bookkeeping
  ├─ refuse any slice whose gate verdict is missing, red or for another sha
  ├─ git merge --no-ff each; stamp its run receipt with the landing sha; stop at a conflict
  ├─ commit the stamped receipts
  ├─ full wave gate on merged main ── red: keep every worktree, print `revert <base>`, exit 1
  ├─ slice-worktree drop for each landed slice
  ├─ `wave repack` and commit the refreshed .ai/tickets/wave.lock
  └─ push
```

- `revert <sha>` reverts every commit after the given green sha, parent 1 for merges, and leaves
  the slice branches in place.
- `verified <sha>` writes the full sha to `.ai/artifacts/last-verified`.
- `wave --close [--summary <text>] [--tickets <ids>] [--dry-run]` targets the oldest pending
  emptied entry of the lock. It refuses unless every ticket of that entry is shipped, a verifier is
  recorded at or after the last landing, and the full wave gate passes on `main`.
  `close_ceremony` then builds the marker commit as an unreachable object, runs the marker
  checks of `super::base` on it, and fast-forwards `main` to it only if they accept. It then
  repacks the lock and commits the refresh. `--dry-run` prints the subject and writes nothing.

## Boundaries

- Depends on: `super::gate` (`cmd_gate`), `super::verdict` (`land_refusal`), `super::base`
  (marker checks), `super::ledger`, `super::push`, `crate::commands::platform::slice_worktree`
  (`drop`), `ticket_engine` (`metrics`, `wave_lock`, `repository`), and `git`.
- Used by: `tools_v2/xtask/src/commands/platform/wave_execution/flush.rs` (the `land`, `revert`,
  `verified` and `wave --close` dispatch).
- Rules: every argument parser is an allowlist, since a filter that is ignored lands more than was
  asked for; a red gate after merge never drops a worktree; only `close_ceremony` writes marker
  commits, and what it checked is what lands, by sha; the tests are in
  `tools_v2/xtask/src/commands/platform/wave_execution/tests/land/tests.rs`.

## Related documentation

- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — landing, shipping, the verifier record
  and the close, in order.
