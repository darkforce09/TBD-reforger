**Status:** live

# Factory waves

How the platform factory turns planned [tickets](/documentation_v2/glossary/n_to_z.md#ticket) into
shipped code: an orchestrating session (the orchestrator) takes one
[wave](/documentation_v2/glossary/n_to_z.md#wave) of file-disjoint tickets at a time, gives each to a
slice agent in its own git worktree, lands the gate-green slices on `main`, has one adversarial
verifier attack the result, and closes the wave with a marker commit. `cargo xtask platform wave`
automates the mechanics; these runbooks hold the procedure and the rules, which are
operator-defined and binding. Read them before dispatching any slice agent.

## Contents

```text
documentation_v2/runbooks/factory_waves/
├── adversarial_verifier_brief.md   the verifier's brief, the severity table and triage
├── cold_start_and_preflight.md     starting a session: services, reclaim, preflight, status
├── known_traps.md                  the signature defect, the perturbation habit, and every trap
├── running_a_wave.md               one wave end to end: worktrees, gates, land, ship, close
├── slice_agent_brief.md            the slice brief, the report schema and the reject conditions
└── wave_planning.md                the wave lock, `owns`, collisions, width and numbering
```

## How it works

```text
main ──┬── slice/<A> worktree ─▶ slice agent A ─┐
       ├── slice/<B> worktree ─▶ slice agent B ─┼─ each: implement, prove, commit,
       ├── slice/<C> worktree ─▶ slice agent C ─┘   gate --slice, report
       ├── land: merge each gate-green slice ─▶ wave gate on merged main ─▶ drop ─▶ repack ─▶ push
       ├── adversarial verifier on merged main ─▶ triage: fix BLOCKERs, defer the rest
       └── ship + stamp ─▶ repack ─▶ verified ─▶ wave --close (marker commit) ─▶ next wave
```

The orchestrator never implements. It plans, dispatches, integrates, verifies, sequences and owns
every ticket status change; everything under `apps/`, `contracts_v2/`, `assets_v2/` and
`tools_v2/` is written by an agent in a worktree. That keeps the orchestrator's context clear and
gives each ticket a whole context of its own. The rules every wave follows:

1. **One worktree per ticket.** `slice-worktree new` creates it from `main`; a sub-slice (two
   dots) shares its parent's worktree.
2. **Concurrency is file-disjointness, computed, never guessed.** `cargo xtask slice-collisions`
   packs tickets whose `owns` lists do not overlap, up to the wave's width
   ([Wave planning](/documentation_v2/runbooks/factory_waves/wave_planning.md)).
3. **Tiered gates.** A slice pays the cheap slice gate in its worktree; the full wave gate runs
   once on merged `main`. `cargo xtask ci ci-local` is not a wave step.
4. **Land only reported, gate-green slices.** `land` merges each slice whose tree is committed
   and clean and whose gate verdict receipt matches its tip; the orchestrator waits until every
   agent of the wave has reported, because a clean tree does not mean a finished agent.
5. **One adversarial verifier per wave**, on merged `main`, after the last landing. Its findings
   are triaged by table: BLOCKERs are fixed in the wave, everything else is filed `deferred`.
6. **Push after every landing.** `land` ends with `platform wave push`, so work is never trapped
   on one machine.
7. **Agents never ship.** They implement, prove by perturbation, gate and report; the
   orchestrator ships, stamps, repacks, records the verifier and closes the wave.
8. **Agents leave their tree clean** and put throwaway probes in `/tmp`, never in the source tree.
9. **Every agent runs on the operator's chosen model tier**, never downgraded to get past a rate
   limit, an overload, latency or cost. The verifier runs on a different strong model from the
   slice agents where the orchestrator's tooling allows it.
10. **Verify green, then dispatch the next disjoint set** without waiting to be asked, except
    after a wave that changed what users see: that wave stops for the operator's eye-pass.

**Branches.** `CLAUDE.md` Law 2 puts every commit on `main` and forbids creating branches. Its
one exception is the `slice/<id>` branch that the xtask slice and wave tooling creates and deletes
itself: `cargo xtask platform slice-worktree -- new <id>` creates `slice/<id>` from `main`
(`tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs`); `drop` force-deletes it and
`reap` deletes the merged ones (`tools_v2/xtask/src/commands/platform/slice_worktree/drop.rs`);
`platform wave land` merges them with `--no-ff`
(`tools_v2/xtask/src/commands/platform/wave_execution/land/merge_execution.rs`); and
`cargo xtask mod wave` uses the same names for the [mod](/documentation_v2/glossary/g_to_m.md#mod)
program. No agent and no orchestrator creates a branch by hand, and there are no pull requests.

**Editor pre-close.** A [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) wave runs
`cargo xtask mk leptos-gates` after its wave gate passes and before it closes. The wave gate
deliberately runs no browser, so that command is the only automated run of the editor smokes,
the rect guards among them; adding a browser smoke to `platform wave gate` is not the fix for a
skipped pre-close. [Editor gates](/documentation_v2/runbooks/editor_gates.md) covers the command.

**Known traps.** The recurring defect is a tool reporting success over an input it never
examined, and the habit that catches it is perturbation: break the guarded code, watch the check
go red, restore, touch the file, watch it go green.
[Known traps](/documentation_v2/runbooks/factory_waves/known_traps.md) lists every trap that has
cost the factory time, from the shared cargo cache to `rg` existing only inside agent shells.

Run the topic runbooks in this order:

| Order | Runbook | When |
|---|---|---|
| 1 | [Cold start and preflight](/documentation_v2/runbooks/factory_waves/cold_start_and_preflight.md) | at the start of every orchestrating session |
| 2 | [Wave planning](/documentation_v2/runbooks/factory_waves/wave_planning.md) | before promoting tickets, or when the plan looks wrong |
| 3 | [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) | every wave |
| 4 | [Slice agent brief](/documentation_v2/runbooks/factory_waves/slice_agent_brief.md) | dispatching each slice and reading its report |
| 5 | [Adversarial verifier brief](/documentation_v2/runbooks/factory_waves/adversarial_verifier_brief.md) | once per wave, after the last landing |
| — | [Known traps](/documentation_v2/runbooks/factory_waves/known_traps.md) | before a first wave, and whenever a green looks too easy |

Stop and ask the operator, rather than improvise, when two slices' merges conflict (the `owns`
computation should have prevented it), when the wave gate is red for a reason that cannot be
attributed, when a BLOCKER needs more agents than the wave planned, when a ticket turns out to be
stale, when the disk falls below 20 GB, when data loss is possible, and when the orchestrator is
about to edit application code itself.

## Code

- [Platform factory commands](/tools_v2/xtask/src/commands/platform/README.md) — the `platform`
  group: `slice-worktree`, `preflight`, `wave` and `slice-run`.
- [Platform wave driver](/tools_v2/xtask/src/commands/platform/wave_execution/README.md) —
  `platform wave`: status, gates, land, close, push, run and test.
- [Platform wave gate drivers](/tools_v2/xtask/src/commands/platform/wave_execution/gate/README.md)
  — the slice gate and the wave gate, step by step.
- [Platform wave landing and close](/tools_v2/xtask/src/commands/platform/wave_execution/land/README.md)
  — `land`, `revert`, `verified` and `wave --close`.
- [Wave gate base derivation](/tools_v2/xtask/src/commands/platform/wave_execution/base/README.md)
  — the marker grammar and the oracles behind the gate's base.
- [Slice worktree lifecycle internals](/tools_v2/xtask/src/commands/platform/slice_worktree/README.md)
  — `new`, `list`, `merge`, `drop` and `reap` with their guards.
- [Platform factory preflight checks](/tools_v2/xtask/src/commands/platform/preflight/README.md)
  — every preflight check.
- [Wave lock command group](/tools_v2/xtask/src/commands/wave/README.md) and
  [Wave lock](/tools_v2/ticket-engine/src/wave_lock/README.md) — the plan and its compiler.
- [Ticket command group](/tools_v2/xtask/src/commands/ticket/README.md) — `ship`, `stamp-sha`,
  `add` and `set-status`.

## Boundaries

- Depends on: the [runbook template](/documentation_v2/standards/templates/runbook.md); the xtask
  `platform`, `wave`, `ticket`, `mk` and `db` command trees; the ticket files and
  `.ai/tickets/wave.lock`; `CLAUDE.md` Law 2.
- Used by: `cargo xtask platform wave`, whose help names this README
  (`PLATFORM_FACTORY_RUNBOOK` in `tools_v2/xtask/src/core/repository_layout.rs`); code comments in
  `tools_v2/xtask/src/commands/platform/`, `build/recipes/shell_word.rs` and
  `agent_context/guards.rs`; tickets that cite this README; the READMEs of the code folders
  above, `tools_v2/`, the ticketboard and the ticket engine; the glossary's wave entry; the
  editor gates, testing and CI and mod slice workflow runbooks; `.cursor/rules/`.
- Rules: this README keeps its path, because the xtask layout pins it and tickets cite it; a
  topic file stays at or under 500 lines; every command in a topic file is checked against the
  xtask source; the runbooks name no agent product and say "orchestrator" for the orchestrating
  session, since the glossary's command center is the web dashboard.

## Related documentation

- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the same worktree
  cycle for the mod program, driven by `cargo xtask mod wave`.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the Mission Creator pre-close.
- [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md) — every check, and which of them
  the slice and wave gates run.
- [Ticket registry](/.ai/tickets/README.md) — the ticket files, their fields and statuses.
- [Factory run archive](/documentation_v2/archive/factory_runs/platform_factory_2026_08.md) — the
  dated handoffs, backlogs, costs and run logs these runbooks replace, frozen with their siblings
  in the same folder.
