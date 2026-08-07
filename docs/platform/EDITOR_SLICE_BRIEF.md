# Editor factory — slice-agent brief

**This file exists because the block below did not.** Through editor waves 100–120 the standing
HARD RULES were retyped from the orchestrator's working context on every dispatch; the only written
trace was a 49-word parenthetical in `.ai/artifacts/editor_factory_run.md`. A genuinely fresh
orchestrator could not reconstruct them — the single largest gap in the continuation recipe.
Recorded verbatim 2026-08-07 from the wave-106 dispatch (T-647).

**It is not a token optimisation.** Measured against the 13-wave run: the rules block is ~290
tokens, base context is 220,587,879 token-turns for 22,274 tokens, so one base token costs ~9,903
token-turns → the block is **2,871,870 token-turns = 0.12%** of the program. Replacing it with a
file the agent must `Read` costs a turn, leaves the same tokens resident, and adds a skip-the-read
failure mode. **Keep pasting it inline.** This file is the source of truth for *what* to paste, so
the wording cannot drift or be lost.

---

## HARD RULES — paste verbatim into every slice dispatch

```
HARD RULES:
1. NO sub-agents. 2. NO .py files committed. 3. Bare `cargo` fails (GLIBC): distrobox-host-exec sh -c 'cd <worktree> && CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target cargo <cmd>' — never /tmp. wasm32 check; native cargo test -p website-frontend.
4. Commit on slice/T-xxx; explicit paths; subject "T-xxx:"; end message with:
Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
5. Before reporting: CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target bash scripts/platform/wave.sh gate --slice T-xxx from the worktree — must end SLICE GATE: PASS. Gate lock WAITING = serialisation, not a hang.
6. No make schema-validate in worktrees. 7. No doc/ticket/plan edits. 8. Tree clean at the end.
9. CONTEXT DISCIPLINE — these are enforced by a PreToolUse hook, not by good intentions:
   - Locate before you read: Grep for the symbol, then Read with offset/limit. A whole-file Read
     of anything over ~400 lines is refused; pass offset or limit and it is allowed.
   - Never re-read a path you have already read in full. Scroll back. A ranged re-read is legal.
   - Never run an uncapped `rg`/`grep` in Bash — use the Grep tool (it caps with head_limit and
     honours .gitignore), or append `| head -50`.
   - Run noisy builds through the filter: `cargo xtask ai run -- '<cmd>'`. It drops passing tests
     and progress chatter, and always passes failures and the gate verdict through unchanged.
```

Rule 9 is measured, not stylistic. Over one 13-wave run, `Read` results were **43.3%** and `Bash`
results **17.4%** of all 2,355,581,888 input-side tokens; 618 of 987 `Read` calls (63%) were
re-reads of a path the same agent had already read.

## Per-ticket content (the only part that changes)

| Component | Source | Notes |
|---|---|---|
| worktree path + branch | derived from `T-xxx` | `.ai/artifacts/worktrees/T-xxx`, branch `slice/T-xxx` |
| `TICKET` / `SPEC` | `.ai/tickets/registry.json` `summary`, **verbatim** | the registry summary *is* the handoff |
| `OWNS (touch ONLY)` | `wave_plan.tsv` column 4 | load-bearing — the only thing preventing two agents colliding on one file |
| forward constraints | `editor_factory_run.md` §constraints | pre-written, keyed by ticket |
| sibling `owns` | `wave_plan.tsv`, same wave | so the agent knows what it must not touch |

## FINAL REPORT — required schema

Return **exactly these fields**. The seven reject conditions in
[`FACTORY_FOR_CURSOR.md`](FACTORY_FOR_CURSOR.md) §Reject are presence checks against this schema,
so a missing field is a structural failure rather than a judgement call. Prose padding around the
fields is what the orchestrator pays for on every subsequent turn — omit it.

```
pwd_branch:            <output of `pwd && git branch --show-current`>
defect_verified_on_main: [ {claim, path:line} ]   # what you confirmed still broken BEFORE coding
changes:               [ {path, line, why} ]
perturbation:          { red_output: <VERBATIM red>, restored_green: true|false }
gate_verdict_tail:     <pasted verbatim, must end SLICE GATE: PASS>
files_outside_owns:    [ {path, why} ]            # empty list if none — never omit the field
found_not_fixed:       [ {path:line, repro} ]     # you report; you do NOT file tickets
deviations:            [ ... ]                    # incl. anywhere you contradict this brief
commits:               [ <sha> ]
```

**A test that printed `skip:` is a FAIL, not a pass.** **`perturbation.red_output` must be the real
captured output** — "I verified it works" is not evidence, and a slice without it is asserted, not
verified. After restoring a perturbation, `touch` the file before re-running: a `git checkout`
restore does not reliably re-trigger a cargo rebuild, so you can get the perturbed verdict back
over correct source.
