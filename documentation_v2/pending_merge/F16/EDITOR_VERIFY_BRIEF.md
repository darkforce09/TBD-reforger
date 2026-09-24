# Editor factory — adversarial verifier brief

The editor program had no verifier template of its own; the only stored one
(`docs/mod/VERIFY_AGENT_PROMPT.md`) is for the mod program and is never cited from the editor
recipe, so each wave's verifier brief was reconstructed from
[`FACTORY_FOR_CURSOR.md`](FACTORY_FOR_CURSOR.md) §7. Recorded here 2026-08-07.

**Model: Fable 5, every wave, no exceptions.** Operator decision 2026-08-07, after costing the
alternative: the 13 full-wave verifiers averaged 26.8M tokens each (14.8% of the program) and the
5 focused re-verifiers averaged 6.7M — a 4× saving that was **declined**. Verification is not
tiered and not traded. The verifier runs once per wave, on merged `main`, after the last landing.

## Why this step is not optional

It has repeatedly produced more value than the slices themselves. It found a gate that passed code
it never compiled, and it twice *disproved* a command-center worry — worth as much. **A verifier
that finds nothing is not wasted.**

## Brief it with

- The wave base sha and the three merge shas.
- The highest-risk claim each agent made — **especially anything it admitted was untested**.
- An instruction to **restore anything it mutates**.
- The severity table below, which is also the orchestrator's triage table.

## Severity

| Severity | Definition |
|---|---|
| **BLOCKER** | `main` is broken, data is at risk, **or a gate reports success on code it never examined** — that last one is always a BLOCKER regardless of how small it looks |
| **MAJOR** | A shipped ticket does not do what it claims, or it can destroy operator-authored work |
| **MINOR / NIT** | Everything else |

Triage: **BLOCKER** → fix in this wave, the wave does not close. **MAJOR** → fix in-wave only if it
can lose authored work or blocks a feature, else file `deferred`. **MINOR/NIT** → file `deferred`
immediately, never create wave work. Calibration: of 46 findings in one platform wave, five were
kept and forty-one deferred.

## Standing instruction — paste verbatim

```
Do NOT fix. Do NOT commit. Do NOT file tickets. Leave main exactly as you found it.
End with an explicit list of what you attacked and FAILED to break — that tells me what
nobody needs to re-audit.

CONTEXT DISCIPLINE (enforced by a PreToolUse hook, not by good intentions):
- Grep for the symbol, then Read with offset/limit. Whole-file Reads over ~400 lines are refused.
- Never re-read a path you already read in full; a ranged re-read is legal.
- No uncapped rg/grep in Bash — use the Grep tool, or append `| head -50`.
- Noisy builds go through `cargo xtask ai run -- '<cmd>'`; it never hides a failure or a verdict.
```

## Report shape

Per finding: **Evidence → Impact → Disposition**, each as
`SEVERITY | file:line | what is wrong | how you proved it`.

Then one line: **is `main` safe to build the next wave on — yes or no?**

Close with a **verified-clean register**: claims you re-proved rather than took on trust. If you
found nothing in a category, say which falsification attempts you made — vague reassurance is a
failed verification.

Report → `.ai/artifacts/editor_verify/wave<L>.md`. Then the orchestrator runs
`cargo xtask platform wave verified $(git rev-parse HEAD)`; the close refuses if any commit
landed after the verifier ran, because the verifier examines **merged main**.
