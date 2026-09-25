# Wave gate base derivation

How the platform wave gate finds and checks the commit its change-scoped steps diff against: the
last `wave N CLOSED` marker commit, verified to cover the whole
[wave](/documentation_v2/glossary.md#wave) before any step trusts it.

## Contents

```text
tools_v2/xtask/src/commands/platform/wave_execution/base/
├── demand_base_confirmation.rs  base coverage checks, the operator confirmation and the empty-range refusal
└── wave_close_number.rs         the marker parser, revert detection, previous-close lookup and the three oracles
```

## How it works

`tools_v2/xtask/src/commands/platform/wave_execution/base.rs` re-exports the history readers of
`ticket_engine::wave_lock::history` and the functions of both files.

- A marker is a commit whose subject is `wave <n> CLOSED`, ended there or followed by `:`, ` —`
  or ` -`; `wave_close_number` accepts nothing else. `wave_close_disavowed` skips a marker that a
  later `git revert` names in its `This reverts commit <sha>.` trailer, and `prev_wave_close`
  returns the newest marker still standing.
- `gate_base_covers_wave` refuses (exit 2) a base that is not an ancestor of `HEAD`. It then
  cross-checks the derived marker with three oracles: the marker claims the highest wave number
  reachable, by exactly one (`wave_close_is_newest_wave`); the ticket ledger at the marker does
  not contradict that the wave's tickets shipped (`wave_close_ledger_says`); and the base does not
  cut through the wave's slice merges (`slice_span_check`, which needs no marker).
- When nothing can corroborate the base, `demand_base_confirmation` prints it and refuses unless
  `TBD_GATE_BASE_CONFIRM` names that sha.
- `refuse_empty_range` refuses a range with no commits, such as `main...HEAD` run from `main`
  instead of a slice worktree.

There is no `HEAD~1` fallback: after several merges that range covers only the last one.

## Boundaries

- Depends on: `super::ledger` (plan and registry reads), `super::git_stdout`,
  `ticket_engine::wave_lock::history` and `ticket_engine::wave_lock::archived_wave_plans`, and
  `git`.
- Used by: `super::gate` (`cmd_gate`, `gate_slice`), `super::land` (`wave --close` and the close
  ceremony, which runs the marker checks on its candidate commit) and `super::verdict`.
- Rules: the marker grammar stays anchored, since a looser one lets a subject such as
  `wave 76 CLOSED? reopened` become a base; a base that cannot be verified refuses rather than
  passes; the tests are in `tools_v2/xtask/src/commands/platform/wave_execution/tests/base/tests.rs`.

## Related documentation

- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — the gate base in the wave procedure, and
  when to confirm one with `TBD_GATE_BASE_CONFIRM`.
