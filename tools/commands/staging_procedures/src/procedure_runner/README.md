# Staging procedure engine

The generic engine every staging procedure runs on: the step vocabulary a procedure fills, the
checks its plan must pass, the runner that awaits each effect against its deadline, and the
recorded run that binds one run to its receipt.

## Contents

```text
tools/commands/staging_procedures/src/procedure_runner/
├── clock.rs          `WaitingClock`: the workspace clock (`time_source`) extended with waiting
├── mod.rs            the module tree
├── probe_reading.rs  one observation: host read or browser entry, judgement, journal entry
├── procedure.rs      the `StagingProcedure` trait, `ProcedurePlan` checks and manifest definition
├── recording.rs      `record`: plan check, `begin`, run folder, identities, run, `finish`
├── runner.rs         `ProcedureRunner`: AWAIT lines, host actions, request rows, deadlines, cases
└── step.rs           steps, kinds, probes, verdicts, deadlines, effects and declared cases
```

## How it works

A procedure returns a `ProcedurePlan`: its declared cases (a case the environment cannot run
carries its missing dependency and is recorded `NOT RUN`), its steps, a hard stop and a poll
interval. Each `Step` has an id, a kind (`chrome_action`, `host_action` with its command, or
`observation`), the `AWAIT` instruction, an optional request predicate, and effect predicates. An
effect has a probe (a host read or the step's browser inbox entry, and a judge), a deadline
counted from the observed request row or from the step's start, and the case it decides.

For each step the runner prints `AWAIT <step>: <instruction>` (and where a browser read goes), runs
the host action, polls the request probe for up to 900 s and takes the row's own time as the
anchor, then polls every effect until it is satisfied, contradicted, or past its deadline or the
hard stop. A case is `ok` only when every effect mapped to it was satisfied in time; a miss or a
contradiction fails it with the reason and what was last seen. Every observation is journaled; the
deciding ones become the log's `observation:` lines. Probes read only: a probe whose command is not
a read is contradicted before it reaches the host.

`record` refuses a plan with no case, a duplicate, an undeclared or undecided case, a decided
not-run case, or a request-row deadline without a request, before `RecordingSession::begin`.
The receipt's environment is the environment identities plus the procedure's
`staged_preconditions` (`staged_precondition=<name>`), so a precondition set up by a tool rather than by elapsed time is named in the receipt itself.
After `begin` every outcome reaches `finish`: a run that stops with an error becomes a failing
receipt naming it.

## Boundaries

- Depends on: `tools/commands/staging_procedures/src/{observation_journal, remote_observers,
  operator_coordination, environment_identity, procedure_receipts}`.
- Used by: the three procedure modules and `tools/commands/staging_procedures/src/staging_dispatch.rs`.
- Rules: the engine never reads stdin.
