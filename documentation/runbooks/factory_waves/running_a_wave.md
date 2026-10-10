**Status:** live

# Running a wave

Runs one [wave](/documentation/glossary/n_to_z.md#wave) of the platform factory end to end: prove the
tickets are not stale, create one worktree per [ticket](/documentation/glossary/n_to_z.md#ticket),
dispatch a slice agent into each, land the gate-green slices on `main`, run one adversarial
verifier, ship the tickets and close the wave with its marker commit. The orchestrator runs every
step from the main checkout unless a step says otherwise. A wave takes hours of agent time and a
few minutes of orchestrator commands per step.

```text
ttm wave collisions ─▶ staleness check ─▶ slice-worktree new (one per ticket)
                                             │
            slice agents, in parallel, each in .ai/artifacts/worktrees/<id> on slice/<id>
            (implement ─▶ perturbation proof ─▶ commit ─▶ gate --slice <id> ─▶ report)
                                             │
land ─▶ merge + ttm land each ─▶ wave gate on merged main ─▶ drop worktrees ─▶ ttm wave repack
  ─▶ push ─▶ (Mission Creator waves: mk leptos-gates) ─▶ adversarial verifier ─▶ triage and fixes
  ─▶ ttm ship each ─▶ ttm wave repack ─▶ verified ─▶ wave --close (marker + ttm wave close)
  ─▶ push ─▶ reclaim ─▶ preflight
```

## Prerequisites

- [Cold start and preflight](/documentation/runbooks/factory_waves/cold_start_and_preflight.md)
  done: `PREFLIGHT: PASS`, the database, the API and the app up.
- The wave's tickets planned as [Wave planning](/documentation/runbooks/factory_waves/wave_planning.md)
  describes: dispatchable, with exact `owns` lists, and file-disjoint, in the wave plan of the
  central ticket manager (`ttm`, project `reforger`).
- The [slice agent brief](/documentation/runbooks/factory_waves/slice_agent_brief.md) and the
  [adversarial verifier brief](/documentation/runbooks/factory_waves/adversarial_verifier_brief.md)
  to hand.

## Steps

1. Take the next dispatch set.

   ```bash
   ttm --project reforger wave collisions
   ```

   Expected: `next wave is <n>. Max disjoint dispatch set (<k>, cap <width>):` and the tickets with
   their `owns`; `cargo xtask platform wave status` names the same command. Dispatch exactly these;
   the width is a limit on how many reports the orchestrator can read and act on, not on the
   machine.

2. Prove each ticket is not stale. Print its record, then open every file its summary cites and
   confirm the defect is still there on `main`, with your own reading of the code: a string match
   is not proof, since a grep hit can be prose while the real definition lives in another file.

   ```bash
   ttm --project reforger show <ticket>
   ```

   Expected: the ticket's record; a slug (`slot-identity.flatten-emit`) or a legacy number
   (`T-674.1`) both resolve. A ticket whose defect is already fixed is not dispatched: ship it with
   the commit that fixed it (`ttm --project reforger ship <ticket> --sha <fixing sha>`), tell the
   operator, and take the next ticket of the plan. A large share of the `idea` backlog is stale at
   any time, so this check runs every wave, and a ticket that turns out to be wrong is corrected in
   place.

3. Create one worktree per ticket; a sub-slice of three dot segments shares its two-segment
   parent's worktree.

   ```bash
   cargo xtask platform slice-worktree -- new <ticket id>
   ```

   Expected: `  oracle ok: mod/<lane> -> <path>` for each lane, the note that `assets/terrains`
   holds LFS pointers in the worktree, and
   `worktree: <repo>/.ai/artifacts/worktrees/<ticket>   branch: slice/<ticket>`, named by the slug
   or the legacy number given. The subcommand is `new`; any other word prints the usage and exits 2.

4. Dispatch one slice agent per worktree, in parallel, with the
   [slice agent brief](/documentation/runbooks/factory_waves/slice_agent_brief.md) filled in.
   Either run the agent through the tooling, which records the run receipt `land` expects:

   ```bash
   cargo xtask platform slice-run <ticket id>
   ```

   Expected: the agent command (`TBD_SLICE_RUN_AGENT_CMD`) runs in the worktree on the prompt
   `ttm brief <ticket>` prints, and `[<ticket>] receipt <receipt id>` reports the run receipt
   recorded in the ticket manager with `ttm record-run` (`ttm --project reforger metrics <ticket>`
   lists it); an answer without a usage object fails and records nothing. A programme resolves to
   its active slice. It refuses a ticket whose executor is not `claude-code` or whose spec the
   ticket manager does not hold. Or dispatch through the orchestrator's own agent tooling, and land
   in step 7 with `--bookkeeping`. Either way every agent's first action is
   `pwd && git branch --show-current`; an agent that reports the main checkout and `main` has not
   entered its worktree and stops before it edits anything.

5. Each agent gates its own slice from inside its worktree before it reports.

   ```bash
   cargo xtask platform wave gate --slice <ticket id>
   ```

   Expected: one `PASS` or `FAIL` line per step, then `SLICE GATE: PASS`, and a verdict receipt at
   `.ai/artifacts/verdicts/<ticket id>.json` in the main checkout, stamped with the slice's tip
   commit. Run from `main` it refuses with exit 2 (an empty `main...HEAD` range). The
   [gate README](/tools/commands/platform_execution/src/wave_execution/gate/README.md) lists the
   steps of both gates side by side.

6. Accept or reject each report with the reject table of the
   [slice agent brief](/documentation/runbooks/factory_waves/slice_agent_brief.md), then check
   its claims against the branch. Wait for every agent of the wave to report before landing any
   of them: `land` checks that a tree is committed and clean, which is not the same as the agent
   being finished, and a slice merged under a live agent deletes its worktree mid-run.

   ```bash
   git log --oneline main..slice/<ticket id>
   ```

   Expected: the agent's commits, subjects starting `<ticket id>:`. Then
   `git diff --stat main..slice/<ticket id>` lists only the files the report names, and
   `git -C .ai/artifacts/worktrees/<ticket id> status --porcelain` prints nothing.

7. Land the wave.

   ```bash
   cargo xtask platform wave land
   ```

   Expected: `gate verdict PASS: <id>@<sha> …`, `revert target: <sha>`, one
   `── landing <id>: <title>` per slice with `landing recorded @ <sha> (receipt stamped landed)`
   (`ttm land <slice> --sha <HEAD>`, which stamps the newest receipt `landed` and ships nothing),
   `landed <k> slice(s). Running the wave gate on merged main:`, the gate's steps and `GATE: PASS`,
   the worktree drops, `wave plan repacked: <summary>` (`ttm wave repack`; nothing is committed),
   and the push. Add `--bookkeeping` when the agents ran outside `slice-run` and left no receipts in
   the ticket manager; name tickets (slugs or legacy numbers) to land only those; `--wave` holds
   every ready slice while any slice of the wave is unfinished. `land` lands only tickets of the
   current wave and refuses the whole run (exit 2) when any slice lacks a green verdict for its tip,
   so nothing lands half-examined.

8. For a [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) wave, run the editor
   gates after the wave gate and before the close: the wave gate runs no browser, so this is the
   only automated run of the editor smokes, the rect guards among them.

   ```bash
   cargo xtask mk leptos-gates
   ```

   Expected: the release build, the gate doctor and the editor suite, in order, exit 0.
   [Editor gates](/documentation/runbooks/editor_gates.md) reads each result.

9. Dispatch one adversarial verifier against merged `main` with the
   [verifier brief](/documentation/runbooks/factory_waves/adversarial_verifier_brief.md), and
   triage what it finds with the table there. A BLOCKER is fixed in this wave by a fix agent in a
   slice worktree (steps 3 to 7), and the fix commit gets a focused re-verify; every other
   finding is filed as a `deferred` ticket with a copy-pasteable repro.

10. File each finding that is not fixed now as a ticket.

    ```bash
    ttm --project reforger add --title "<title>" --summary "<the finding, file:line and repro>" --owns <path>
    ```

    Expected: `Created work ticket <slug>`, status `idea`; pass `--owns` once per file. Set it
    `deferred` with `ttm --project reforger set-status <ticket> deferred`, and confirm with
    `ttm --project reforger check`. Agents never file tickets; the orchestrator does, and checks
    every report claim of the form "I filed" or "a sibling fixed this" against the ticket manager
    and `git log` before acting on it.

11. Confirm what landed. Each landed ticket has a merge commit whose subject is
    `<ticket id>: <title>`; that merge is its landing commit, which `land` recorded in the ticket
    manager.

    ```bash
    git log --merges --format='%h %s' -n 20
    ```

    Expected: one merge line per landed ticket of the wave.

12. Ship each landed ticket; the plan is repacked once, in step 13.

    ```bash
    ttm --project reforger ship <ticket> --no-repack
    ```

    Expected: `<ticket> shipped`. Repeat for every landed ticket of the wave.

13. Repack the plan once, now that the whole wave is shipped.

    ```bash
    ttm --project reforger wave repack
    ```

    Expected: `wave plan: <summary>`; the plan gains a pending emptied entry for the wave, which the
    close in step 15 targets. Then `ttm --project reforger check` prints
    `reforger: 0 error(s), <n> warning(s)`. Tickets and the plan live in the ticket manager, so
    nothing is committed: report the gate verdict, what the verifier found, every ticket filed and
    why, and any BLOCKER still outstanding to the operator.

14. Record the verifier. The close refuses when any commit landed after the recorded sha, so
    record it once nothing sits after the tree the verifier examined;
    `git diff --stat <verifier sha>..HEAD` prints nothing.

    ```bash
    cargo xtask platform wave verified $(git rev-parse HEAD)
    ```

    Expected: `recorded: adversarial verifier examined <sha>`, written to
    `.ai/artifacts/last-verified`.

15. Close the wave.

    ```bash
    cargo xtask platform wave wave --close --summary "<one line: what the wave shipped>"
    ```

    Expected: `wave <n>: all tickets shipped ✓`, `wave <n>: verifier examined this exact tree ✓`,
    the full wave gate against the wave's own base, `close subject self-check ✓`,
    `marker committed: <sha> wave <n> CLOSED — <summary>`, then
    `close recorded: wave <n> at <sha> (<k> member(s))` (`ttm wave close <n> --sha <marker>`, which
    repacks the plan), a green `ttm wave check`, and the check that the plan's wave base equals the
    newest marker in git. A failure after the marker says the marker IS committed and prints the
    `ttm --project reforger wave close <n> --sha <marker>` to re-run, which is safe. The marker is
    built as an unreachable commit, checked by the same functions the gate derives its base from,
    and only then becomes `main`; never type a `wave <n> CLOSED` subject by hand. `--dry-run` runs
    every check and the wave gate, then prints `--dry-run: would commit wave-close marker subject:`
    and the subject, and writes nothing, neither to git nor to the ticket manager.

16. Push.

    ```bash
    cargo xtask platform wave push
    ```

    Expected: a plain `git push origin main` when git-lfs is installed. Without git-lfs it pushes
    with `--no-verify` only when no path in `origin/main..HEAD` resolves to `filter: lfs`, and
    refuses when it cannot tell.

17. Clean up and return to a green preflight.

    ```bash
    cargo xtask platform wave reclaim
    ```

    Expected: the landed slices' private build folders removed. Then
    `cargo xtask platform slice-worktree -- reap` removes any merged, clean worktree `land` did not
    drop, and `cargo xtask platform preflight` prints `PREFLIGHT: PASS`.

18. For a wave that changed what users see, stop here and hand the operator an eye-pass checklist:
    one human-runnable step per ticket, derived from its acceptance text, on the release build
    (`cargo xtask mk leptos`) at 1920×1080, plus "anything that feels wrong is a finding". The
    operator's verdict gates the next wave; eye-pass findings become tickets and are never fixed
    ad hoc in the closed wave. A finding that is a design correction rather than a bug is recorded
    verbatim on its ticket as the operator's decision, with what it supersedes.

## Verify

```bash
cargo xtask platform wave wave
```

Expected: the next wave's header and `STATUS: wave <n+1> is OPEN — …` (or `all waves shipped`),
`verify debt: 0`; `git log -1 --format=%s` shows `wave <n> CLOSED — <summary>`, and
`ttm --project reforger wave check` prints `wave plan OK`.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `land: no gate verdict for <id> under .ai/artifacts/verdicts/ — no gate has run on it` | the agent never ran the slice gate, or ran it from the main checkout | from the worktree: `cargo xtask platform wave gate --slice <ticket id>`, then land again |
| `land: the gate for <id> ran on <sha> but <sha> is being landed — the verdict is STALE` | the slice gained a commit after its gate | re-gate from the worktree |
| `land: the last gate for <id> was FAIL …` | the slice gate failed | the agent fixes the slice and re-gates |
| `land: no slice-run receipt in the ticket manager for: …` | the agents ran outside `platform slice-run` | `cargo xtask platform wave land --bookkeeping`; it stamps only receipts that exist |
| `landing record FAILED for <id>: …` | the ticket manager refused `ttm land` after the merge | the merge is on `main`; fix the cause, run the printed `ttm --project reforger land <id> --sha <sha>`, then land again |
| `land: <ids> not in wave <n> — nothing named was landed` | the ticket was promoted out of plan order | leave the guard alone; repack (`ttm --project reforger wave repack`) so the ticket sits in the current wave, or merge it by hand with `cargo xtask platform slice-worktree -- merge <ticket id>` (which also demands a green verdict) and run `cargo xtask platform wave gate` on `main` |
| `GATE RED AFTER MERGE — all <k> worktree(s) KEPT for inspection` | a gate step failed on merged `main` | read each `FAIL` block; fix on `main` and run `cargo xtask platform wave gate`, or roll back with `cargo xtask platform wave revert <revert target>`, which keeps the slice branches and prints the `ttm --project reforger unland <slice>` that clears each reverted landing record |
| `REFUSING to call this a pass: <n> DB-backed test(s) SKIPPED.` | the gate database was unreachable, so the API tests skipped | `cargo xtask db up`, then re-run the gate; a skip is never a pass |
| `gate: nothing could corroborate this wave base — refusing to run unconfirmed.` | no marker, plan row or slice merge span confirms the derived base | read the printed base; if it is the last real close, re-run with `TBD_GATE_BASE_CONFIRM=<printed sha>` in front of the same command; never pass a sha from memory |
| `gate: WAITING for the gate lock — this is serialisation, NOT a hang.` | another gate holds the lock | wait; `gate: REFUSING — no lock after <n>s` means a stuck gate |
| `REFUSED: no emptied wave pending — nothing to close.` | the wave was shipped one ticket at a time, or not all of it is shipped | ship the rest; for a dissolved wave, `ttm --project reforger wave repack --reserve <ids…>` ([Wave planning](/documentation/runbooks/factory_waves/wave_planning.md)) |
| `REFUSED: <n> commit(s) have landed since the last verifier saw <sha>.` | a commit landed after `verified` | if it changed no code, record `verified` again at `HEAD`; if code landed, the verifier runs again first |
| `REFUSED: no adversarial verifier recorded.` | step 14 was skipped | run the verifier, then step 14 |
| `REFUSED: the working tree is dirty …` | uncommitted files in the main checkout | commit or remove them; the ceremony writes commits and refuses to sweep up others |
| `recording the close FAILED: …` or `ttm wave check is RED after the close ceremony` | the ticket manager refused the close record, or its plan fails the check | the marker IS committed; fix the cause, then run the printed `ttm --project reforger wave close <n> --sha <marker>`, which is idempotent |
| an agent reports `completed` with a rate-limit or reset message | the agent stopped at a limit | treat it as a failed agent; resume it or re-dispatch |

## Related

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the index and the rules.
- [Platform wave driver](/tools/commands/platform_execution/src/wave_execution/README.md) — every
  `platform wave` subcommand.
- [Platform wave landing and close](/tools/commands/platform_execution/src/wave_execution/land/README.md)
  — `land`, `revert`, `verified` and `wave --close` in code.
- [Ticket manager client](/tools/foundation/ticket_manager_client/README.md) — how the drivers
  call `ttm` for the plan, the receipts, `land`, `wave repack` and `wave close`.
- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the same cycle for the
  mod program, driven by `cargo xtask mod wave`.
