**Status:** live

# Factory waves

How the platform factory turns planned [tickets](/documentation/glossary/n_to_z.md#ticket) into
shipped code: an orchestrating session (the orchestrator) takes one
[wave](/documentation/glossary/n_to_z.md#wave) of file-disjoint tickets at a time, gives each to a
slice agent in its own git worktree, lands the gate-green slices on `main`, has one adversarial
verifier attack the result, and closes the wave with a marker commit. `cargo xtask platform wave`
automates the mechanics; these runbooks hold the procedure and the rules, which are
operator-defined and binding. Read them before dispatching any slice agent.

## Contents

```text
documentation/runbooks/factory_waves/
├── adversarial_verifier_brief.md   the verifier's brief, the severity table and triage
├── cold_start_and_preflight.md     starting a session: services, reclaim, preflight, status
├── known_traps.md                  the signature defect, the perturbation spot-check, and every trap
├── running_a_wave.md               one wave end to end: worktrees, gates, land, ship, close
├── slice_agent_brief.md            the slice brief, the report schema and the reject conditions
└── wave_planning.md                the wave plan, `owns`, collisions, width and numbering
```

## How it works

```text
main ──┬── slice/<A> worktree ─▶ slice agent A ─┐
       ├── slice/<B> worktree ─▶ slice agent B ─┼─ each: implement, prove, commit,
       ├── slice/<C> worktree ─▶ slice agent C ─┘   gate --slice, report
       ├── land: merge each gate-green slice ─▶ wave gate on merged main ─▶ drop ─▶ repack ─▶ push
       ├── adversarial verifier on merged main ─▶ triage: fix BLOCKERs, defer the rest
       └── ttm ship ─▶ ttm wave repack ─▶ verified ─▶ wave --close (marker commit) ─▶ next wave
```

The orchestrator never implements. It plans, dispatches, integrates, verifies, sequences and owns
every ticket status change in the central ticket manager (`ttm --project reforger`); everything
under `crates/`, `mod/`, `contracts/`, `assets/` and `tools/` is written by an agent in a
worktree. That keeps the orchestrator's context clear and
gives each ticket a whole context of its own. The rules every wave follows:

1. **One worktree per ticket.** `slice-worktree new` creates it from `main`; a sub-slice of three
   dot segments shares its two-segment parent's worktree.
2. **Concurrency is file-disjointness, computed, never guessed.**
   `ttm --project reforger wave collisions` packs tickets whose `owns` lists do not overlap, up
   to the wave's width ([Wave planning](/documentation/runbooks/factory_waves/wave_planning.md)).
3. **Light gates.** A slice runs check, fmt and the tests of the crates it changed in its
   worktree; the wave gate on merged `main` stays as small. Neither requires the browser gates,
   the documentation gates or a perturbation proof, and `cargo xtask ci ci-local` is not a wave
   step.
4. **Land only reported, gate-green slices.** `land` merges each slice whose tree is committed
   and clean and whose gate verdict receipt matches its tip; the orchestrator waits until every
   agent of the wave has reported, because a clean tree does not mean a finished agent.
5. **One adversarial verifier per wave**, on merged `main`, after the last landing. Its findings
   are triaged by table: BLOCKERs are fixed in the wave, everything else is filed `deferred`.
6. **Push after every landing.** `land` ends with `platform wave push`, so work is never trapped
   on one machine.
7. **Agents never ship.** They implement, gate and report; the orchestrator ships
   (`ttm --project reforger ship`), repacks, records the verifier and closes the wave.
8. **Agents leave their tree clean** and put throwaway probes in `/tmp`, never in the source tree.
9. **Every agent runs on the operator's chosen model tier**, never downgraded to get past a rate
   limit, an overload, latency or cost. The verifier runs on a different strong model from the
   slice agents where the orchestrator's tooling allows it.
10. **Verify green, then dispatch the next disjoint set** without waiting to be asked, except
    after a wave that changed what users see: that wave stops for the operator's eye-pass.

**Branches.** `CLAUDE.md` Law 2 puts every commit on `main` and forbids creating branches. Its
one exception is the `slice/<id>` branch that the xtask slice and wave tooling creates and deletes
itself: `cargo xtask platform slice-worktree -- new <id>` creates `slice/<id>` from `main`
(`tools/commands/platform_execution/src/slice_worktree/git_plain.rs`); `drop` force-deletes it and
`reap` deletes the merged ones (`tools/commands/platform_execution/src/slice_worktree/drop.rs`);
`platform wave land` merges them with `--no-ff`
(`tools/commands/platform_execution/src/wave_execution/land/merge_execution.rs`); and
`cargo xtask mod wave` uses the same names for the [mod](/documentation/glossary/g_to_m.md#mod)
program. No agent and no orchestrator creates a branch by hand, and there are no pull requests.

**Editor smokes.** The wave gate runs no browser. `cargo xtask mk leptos-gates` runs nightly or
on demand; run it before closing a [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
wave only when the wave made a risky editor runtime change.
[Editor gates](/documentation/runbooks/editor_gates.md) covers the command.

**Known traps.** The recurring defect is a tool reporting success over an input it never
examined; when a green looks too easy, break the guarded code once and watch the check go red.
That is a judgment call, not a required proof.
[Known traps](/documentation/runbooks/factory_waves/known_traps.md) lists every trap that has
cost the factory time, from the shared cargo cache to `rg` existing only inside agent shells.

Run the topic runbooks in this order:

| Order | Runbook | When |
|---|---|---|
| 1 | [Cold start and preflight](/documentation/runbooks/factory_waves/cold_start_and_preflight.md) | at the start of every orchestrating session |
| 2 | [Wave planning](/documentation/runbooks/factory_waves/wave_planning.md) | before promoting tickets, or when the plan looks wrong |
| 3 | [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) | every wave |
| 4 | [Slice agent brief](/documentation/runbooks/factory_waves/slice_agent_brief.md) | dispatching each slice and reading its report |
| 5 | [Adversarial verifier brief](/documentation/runbooks/factory_waves/adversarial_verifier_brief.md) | once per wave, after the last landing |
| — | [Known traps](/documentation/runbooks/factory_waves/known_traps.md) | before a first wave, and whenever a green looks too easy |

Stop and ask the operator, rather than improvise, when two slices' merges conflict (the `owns`
computation should have prevented it), when the wave gate is red for a reason that cannot be
attributed, when a BLOCKER needs more agents than the wave planned, when a ticket turns out to be
stale, when the disk falls below 20 GB, when data loss is possible, and when the orchestrator is
about to edit application code itself.

## Code

- [Platform factory commands](/tools/commands/platform_execution/src/README.md) — the `platform`
  group: `slice-worktree`, `preflight`, `wave` and `slice-run`.
- [Platform wave driver](/tools/commands/platform_execution/src/wave_execution/README.md) —
  `platform wave`: status, gates, land, close, push, run and test.
- [Platform wave gate drivers](/tools/commands/platform_execution/src/wave_execution/gate/README.md)
  — the slice gate and the wave gate, step by step.
- [Platform wave landing and close](/tools/commands/platform_execution/src/wave_execution/land/README.md)
  — `land`, `revert`, `verified` and `wave --close`.
- [Wave gate base derivation](/tools/commands/platform_execution/src/wave_execution/base/README.md)
  — the marker grammar and the oracles behind the gate's base.
- [Slice worktree lifecycle internals](/tools/commands/platform_execution/src/slice_worktree/README.md)
  — `new`, `list`, `merge`, `drop` and `reap` with their guards.
- [Platform factory preflight checks](/tools/commands/platform_execution/src/preflight/README.md)
  — every preflight check.
- [Ticket manager client](/tools/foundation/ticket_manager_client/README.md) — how the drivers
  reach `ttm`, which holds the tickets, the run receipts, the wave plan and the close records.

## Boundaries

- Depends on: the [runbook template](/documentation/standards/templates/runbook.md); the xtask
  `platform`, `mk` and `db` command trees; the central ticket manager (`ttm --project
  reforger`), its tickets and its wave plan; `CLAUDE.md` Law 2.
- Used by: `cargo xtask platform wave`, whose help names this README
  (`PLATFORM_FACTORY_RUNBOOK` in `tools/foundation/repository_layout/src/documentation_locations.rs`); code comments in
  `tools/commands/platform_execution/src/`, `build/recipes/shell_word.rs` and
  `agent_context/guards.rs`; tickets that cite this README; the READMEs of the code folders
  above and `tools/`; the glossary's wave entry; the editor gates, testing and CI and mod slice
  workflow runbooks; `.cursor/rules/`.
- Rules: this README keeps its path, because the xtask layout pins it and tickets cite it; a
  topic file stays at or under 500 lines; every command in a topic file is checked against the
  xtask source; the runbooks name no agent product and say "orchestrator" for the orchestrating
  session, since the glossary's command center is the web dashboard.

## Related documentation

- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the same worktree
  cycle for the mod program, driven by `cargo xtask mod wave`.
- [Editor gates](/documentation/runbooks/editor_gates.md) — the Mission Creator pre-close.
- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — every check, and which of them
  the slice and wave gates run.
- [Factory run archive](/documentation/archive/factory_runs/platform_factory_2026_08.md) — the
  dated handoffs, backlogs, costs and run logs these runbooks replace, frozen with their siblings
  in the same folder.
