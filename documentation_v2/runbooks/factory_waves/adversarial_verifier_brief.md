**Status:** live

# Adversarial verifier brief

The brief for the one adversarial verifier each [wave](/documentation_v2/glossary/n_to_z.md#wave) runs
on merged `main`, its severity table, and how the orchestrator triages what it finds. Use it at
step 9 of [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md). The
verifier documents and never fixes; its job is to find what the slice agents got wrong, not to
confirm they were right.

## Why every wave runs one

The verifier has repeatedly found more than the slices it checked: a gate that passed code it
never compiled, new hard failures that refused legitimate work, tests that matched their own
assertion text, a dead-code evaluator that failed open. Its other value is negative: it has
disproved orchestrator worries as often as it confirmed them, so a verifier that finds nothing is
not wasted; its list of what it attacked and failed to break tells everyone what needs no
re-audit. Verification is not tiered or traded against cost: one full verifier per wave, after the
last landing, on merged `main`.

The verifier runs on a different strong model from the slice agents where the orchestrator's
tooling allows it, so it does not share their blind spots; every agent runs on the tier the
operator chose, and none is downgraded to get past a rate limit, an overload, latency or cost.
Retry instead, and ask the operator if a tier change seems warranted.

## Prerequisites

- Every slice of the wave landed and the wave gate green on merged `main`
  ([Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) step 7), and for a
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) wave the editor gates too.
- The wave base (the `revert target:` sha `land` printed) and the merge sha of each slice.
- Each slice report's highest-risk claims, above all anything its agent admitted it did not test.

## The brief

```text
You are the adversarial verifier for wave <n>, on merged main.

Wave base: <revert target sha>. Merges: <ticket id> <merge sha>, one per slice.
Highest-risk claims to attack:
  <ticket id>: <claim, especially anything the agent admitted was untested>

Your job is to find what the slice agents got wrong, not to confirm they were right. Check:
1. Integration seams: the slices were written blind to each other; do their assumptions agree?
2. Claims against reality: re-prove each claim in the reports from the code and by running it.
3. Every new or changed check: break the code it guards and confirm it goes red. A check that
   reports success on code it never examined is always a BLOCKER.
4. Honest failure: bad input gives a clear diagnostic, never a silent half-broken state.
5. Stubs presented as working.

Do NOT fix. Do NOT commit. Do NOT file tickets. Restore anything you mutate and leave main
exactly as you found it. End with an explicit list of what you attacked and FAILED to break.

CONTEXT DISCIPLINE (enforced by the `cargo xtask ai guard` hook):
- Search for the symbol, then read with offset and limit; whole-file reads over 400 lines are
  refused.
- Never re-read a file you already read in full; a ranged re-read is allowed.
- Cap every search's output. Run noisy builds through `cargo xtask ai run -- '<command>'`.

REPORT
Per finding: SEVERITY | file:line | what is wrong | how you proved it.
Then one line: is main safe to build the next wave on — yes or no?
Then the verified-clean register: each claim you re-proved rather than took on trust, and for
each category where you found nothing, the falsification attempts you made.
```

## Severity

| Severity | Definition |
|---|---|
| BLOCKER | `main` is broken, data is at risk, or a gate reports success on code it never examined (always a BLOCKER, however small it looks) |
| MAJOR | a shipped ticket does not do what it claims, or it can destroy work the operator authored |
| MINOR / NIT | everything else |

## Triage

The orchestrator answers these questions in order, literally, for each finding:

```text
Is main broken, is data at risk, or did a gate pass code it never examined?
   yes -> BLOCKER: fix it in this wave. The wave does not close.
   no  -> next question.
Can it destroy work the operator authored, or does it block a feature on the backlog?
   yes -> fix it in this wave.
   no  -> file it as a `deferred` ticket with a copy-pasteable repro. No wave work.
```

- **File deferred, never drop.** A diagnosed, reproducible ticket costs nothing to hold and is most
  of the finding's value; `deferred` tickets never enter a wave until someone promotes them. For
  calibration: one night's reviews produced 46 findings; five were fixed and 41 deferred,
  including an admin-lockout route and four gate defects, all real and none urgent.
- **Small MAJORs.** When a MAJOR is a small mechanical fix whose mechanism the verifier already
  diagnosed, fixing it in the wave and running a focused re-verify of just that commit is cheaper
  than a follow-up wave.
- **Quarantine, don't stop.** A second red on the same ticket reverts that slice
  (`cargo xtask platform wave revert <sha>` keeps the branch), defers its ticket with the full
  diagnosis, and the wave closes with the rest.
- **Two instruments.** On a wave that changed what users see, the verifier catches code rot and
  the operator's eye-pass after the close catches feel; neither replaces the other.

## Steps

1. Dispatch the verifier with the brief above, filled in.
2. Store its report beside the earlier ones under `.ai/artifacts/`
   (`editor_verify/wave<n>.md` for Mission Creator waves); each report's verified-clean register
   is the must-not-break list for the files it names.
3. Triage each finding with the table above; fix BLOCKERs through a fix agent in a slice worktree
   and file the rest ([Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md)
   steps 9 and 10).
4. After the bookkeeping commit, record the tree the verifier examined.

   ```bash
   cargo xtask platform wave verified $(git rev-parse HEAD)
   ```

   Expected: `recorded: adversarial verifier examined <sha>`. `wave --close` refuses while any
   commit sits after this sha, because the verifier must have seen merged `main`.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| the verifier's report has no "failed to break" list | the brief's last instruction was dropped | send it back; vague reassurance is a failed verification |
| the verifier committed or edited files | the brief's standing instruction was not pasted | `git status` in the main checkout; restore its changes, and re-dispatch with the brief |
| `platform wave status` prints `<- OVERDUE, 8+ landings unverified` | landings kept happening with no verifier | run the verifier now, before anything else lands |
| `REFUSED: recorded verify sha <sha> is not an ancestor of HEAD` | the recorded sha is from another branch or a rewritten history | record `verified` again at the tree the verifier examined |

## Related

- [Slice agent brief](/documentation_v2/runbooks/factory_waves/slice_agent_brief.md) — the
  reports the verifier's claims come from.
- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — where the
  verifier sits in the wave.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the verify agent
  prompt of the mod program.
