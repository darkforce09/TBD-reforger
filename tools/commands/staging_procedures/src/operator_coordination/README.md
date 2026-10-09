# Staging operator coordination

What the harness tells the operator and the orchestrator: the numbered list of real actions each
procedure asks approval for (and its recovery list and declared cases), and the lines a recorded
run prints while it waits.

## Contents

```text
tools/commands/staging_procedures/src/operator_coordination/
├── action_list.rs     `PlannedAction` and the rendering of `staging action-list`
├── awaited_effect.rs  the `AWAIT <step>: <instruction>` line, the inbox hint, the outcome lines
└── mod.rs             the module tree
```

## How it works

`staging action-list <procedure>` prints `<check> actions` and one `N. [actor] summary` per action,
with each exact command or click indented under it; `--recovery` prints the recovery list and
`--cases` the declared cases, a not-run case with `NOT RUN (missing: <dependency>)`. A procedure
without actions prints a line saying so. While a recorded run waits it prints one `AWAIT` line per
step, folding any line break of the instruction, then `<step>.<effect> ok (…)` or `FAILED (…)`
per effect.

## Boundaries

- Depends on: `procedure_runner/step.rs` for `DeclaredCase`.
- Used by: the procedures (which supply the lists), `staging_dispatch.rs` and `procedure_runner/runner.rs`.
- Rules: the harness prints and never reads an answer.
