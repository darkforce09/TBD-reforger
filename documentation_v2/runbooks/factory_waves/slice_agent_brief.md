**Status:** live

# Slice agent brief

The brief the orchestrator pastes into every slice agent's dispatch, the report schema the agent
returns, and the conditions under which the orchestrator sends a report back. One slice agent
implements one [ticket](/documentation_v2/glossary/n_to_z.md#ticket) in one worktree and never ships it.
Use this page at step 4 of [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md)
and again at step 6.

## Prerequisites

- The ticket's worktree exists (`cargo xtask platform slice-worktree -- new <ticket id>`).
- The ticket record (`cargo xtask ticket show <ticket id>`): its summary, acceptance text and
  `owns` list are the handoff, written to be pasted verbatim. There is no earlier conversation
  for the agent to read.
- The `owns` lists of every sibling ticket in the same wave, from `cargo xtask slice-collisions`.

## Assembling a brief

Every brief is the template below with the per-ticket parts filled in. The rules block is pasted
inline on every dispatch, never replaced by a pointer to this file: it is a few hundred tokens,
and a brief that asks the agent to read a file first adds a turn and a way to skip the read
without saving anything. This page is the single source of what gets pasted, so the wording
cannot drift.

| Part | Source |
|---|---|
| worktree path and branch | `.ai/artifacts/worktrees/<ticket id>`, branch `slice/<ticket id>` |
| the ticket | the ticket's summary, acceptance and cited `file:line`s, verbatim |
| your files | the ticket's `owns` list |
| sibling files | the other tickets of the wave and their `owns` lists |
| per-ticket constraints | protected code in the ticket's files that an earlier verifier report named |

## The brief

```text
You are the slice agent for ticket <ticket id>.

WORKTREE: work only in <repo>/.ai/artifacts/worktrees/<ticket id> (branch slice/<ticket id>),
never in the main checkout.

FIRST ACTION: run `pwd && git branch --show-current` and paste the output in your report. If it
does not show the worktree path and slice/<ticket id>, stop and report that before editing.

THE TICKET
<summary, acceptance and cited file:line, verbatim>

YOUR FILES (touch only these)
<owns paths>
If the work needs a file outside this list, stop and report which file and why; do not widen
silently. Siblings running now own:
  <sibling ticket id>: <its owns paths>

RULES
1. Verify the defect still exists on main before writing code. If it is already fixed, stop and
   report the commit that fixed it.
2. Prove every test by perturbation: break the code it guards, capture the red output verbatim,
   restore, `touch` the restored file (a restore does not always make cargo rebuild), re-run
   green. Paste the red output in your report.
3. Commit early, on slice/<ticket id>, by explicit path (never `git add -A` or a folder), subject
   "<ticket id>: <what>". Leave the tree committed and clean whenever you pause.
4. cargo, rustfmt and trunk are host binaries. From the development container reach them through
   `distrobox-host-exec`, which forwards no environment: pass variables with `env NAME=value`.
   `cargo xtask platform wave …` bridges itself; run it directly and do not set
   CARGO_TARGET_DIR for it (it drops an inherited one and picks its own folders).
5. Check, clippy and build share the warm cache. Run ad-hoc tests only through
   `cargo xtask platform wave test --slice <ticket id> -p <package> [cargo test arguments]`, which
   builds into a private folder, and compare the `--list` count with the run count every time:
   a mismatch means the binary is not yours. `website-map-engine` needs `--all-features`.
6. A binary you launch (an API instance, a CLI you exercise) builds into its own
   CARGO_TARGET_DIR=<repo>/target-<ticket id>-api; before trusting an HTTP or CLI result, find a
   string unique to your change inside the binary.
7. Gate before reporting, from the worktree:
     cargo xtask platform wave gate --slice <ticket id>
   It must end `SLICE GATE: PASS`. Export
   TBD_GATE_MIGRATION_0016=apps/website/api_v2/migrations/0016_backfill_linked_match_stats.sql
   first. "clippy: REFUSING to pass — … resolved to NO crate" means the diff has no lintable
   Rust, not that the code is broken. `gate: WAITING for the gate lock` is serialisation.
8. In a worktree run `cargo xtask schema validate`, not `cargo xtask ci schema-validate`: the
   terrain assets are LFS pointers here.
9. Never `git stash`. Never run `cargo xtask ci ci-local` (15 to 40 minutes, not a wave step).
   A test that prints `skip:` is a failure, not a pass.
10. Start no sub-agents. Leave the development API on :8080 and the app on :3000 running.
11. You do not ship: no push, no merge, no ticket, plan or documentation edits, no status
    changes. You do not file tickets: report findings with file:line and a repro.
12. Throwaway probes go in /tmp, never in the source tree. Commit no .py file.
13. Read with offset and limit: whole-file reads over 400 lines and re-reads of a file you have
    read are refused by the `cargo xtask ai guard` hook. Run noisy builds through
    `cargo xtask ai run -- '<command>'`; it never hides a failure or a verdict.
14. Measure, do not read: a claim about pixels needs a guard that measures pixels, and a "does
    not reproduce" verdict is only as good as its measurement.
15. If you contradict this brief, say so plainly: you looked, the brief remembered.

REPORT with exactly the fields of the report schema, and nothing around them.
```

## Report schema

A missing field is a structural failure, not a judgement call; the reject table below is a
presence check against these fields.

```text
pwd_branch:              <output of `pwd && git branch --show-current`>
defect_verified_on_main: [ {claim, path:line} ]   confirmed still broken before any code
changes:                 [ {path, line, why} ]
perturbation:            { red_output: <verbatim red>, restored_green: true|false }
gate_verdict_tail:       <pasted verbatim, ending SLICE GATE: PASS>
files_outside_owns:      [ {path, why} ]          an empty list when none; never omitted
found_not_fixed:         [ {path:line, repro} ]   reported, never filed
deviations:              [ ... ]                  including every contradiction of the brief
commits:                 [ <sha> ]
```

## Reject conditions

Send the report back, naming exactly what is missing, when any of these holds. The orchestrator
never fixes a slice itself.

| # | Reject when |
|---|---|
| 1 | there is no pasted red output from a perturbation; "I verified it works" is not evidence |
| 2 | it says the gate passed but does not paste the verdict tail |
| 3 | it claims to have filed a ticket |
| 4 | it treats a test that printed `skip:` as a pass |
| 5 | it touched files outside its `owns` without listing them |
| 6 | it ran `cargo xtask ci ci-local` or `git stash` |
| 7 | it says "already fixed", "a sibling did it" or "this already works" without a command that proves it |

Then check the claims against the branch
([Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) step 6). Agents are
reliable about the code they touched and unreliable about bookkeeping; several agents in one run
reported follow-up tickets that were never filed. When an agent contradicts the orchestrator's
brief, the agent is usually the one that looked: verify with a command, correct the ticket, and
tell the operator.

## The completion pass

An `owns` boundary can stop a slice from finishing work that belongs to its ticket. The rule is
disclose, never defer silently: the slice reports the gap in `found_not_fixed` with a precise
recipe, and after the wave lands, when the sibling files are free, the orchestrator dispatches a
completion agent on merged `main` with the same brief, the recipe as its ticket text and the
now-free files as its `owns`. Completion agents commit a checkpoint after each step.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| the agent's first action shows the main checkout and `main` | the agent did not enter its worktree | stop it before it edits anything; re-dispatch with the worktree as its working folder |
| `cargo test` reports passes but `--list` shows none of the slice's tests | another worktree's binary in the shared cache | rule 5: `cargo xtask platform wave test --slice <ticket id>` |
| `Blocking waiting for file lock`, then `Finished` with no `Compiling` or `Checking` line | cargo replayed a verdict from the shared cache instead of building | treat the result as unproven; use the private test folder, or `touch` the changed file and re-run |
| the gate's fmt step fails on import order | the crate's edition differs (the frontend is 2021, the other crates 2024) | run `rustfmt --edition <the crate's edition>` on the file, or read the gate's output |
| `git status` aborts in the worktree | git-lfs is missing while `filter.lfs.process` is configured | `git -c filter.lfs.process= -c filter.lfs.required=false status`, scoped to the slice's paths |

## Related

- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — where the brief
  is dispatched and the report is read.
- [Known traps](/documentation_v2/runbooks/factory_waves/known_traps.md) — why each rule exists.
- [Slice worktree lifecycle internals](/tools_v2/xtask/src/commands/platform/slice_worktree/README.md)
  — the worktree the agent works in.
- [Agent context guards](/tools_v2/xtask/src/commands/agent_context/README.md) — `ai guard` and
  `ai run`.
