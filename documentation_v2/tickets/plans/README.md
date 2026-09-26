**Status:** live

# Ticket plans

One plan per work [ticket](/documentation_v2/glossary/n_to_z.md#ticket) that went ready: how the work
runs, what can go wrong and how it is verified. The agent working the ticket reads the plan beside
its spec; the spec says what to build, the plan how.

## Contents

```text
documentation_v2/tickets/plans/
└── t-*_plan.md  one plan per ticket, named t-<id>_plan.md
```

## How it works

A plan's name is `t-`, its ticket's id lowercased with dots as underscores, then `_plan.md`: the
plan of ticket `T-<n>.<m>` is `t-<n>_<m>_plan.md`. `plan_path` in
`tools_v2/ticket-engine/src/repository.rs` derives that path, and the ticket's `plan` field in
`.ai/tickets/T-<id>.toml` records it.

A plan is a copy of `.ai/tickets/plan_template.md` with its four sections filled: Context,
Approach, Risks and Verification. Nothing goes ready without one:

```text
cp .ai/tickets/plan_template.md  ─▶  fill the four sections  ─▶  cargo xtask ticket mark-ready <id> <spec> [plan]
                                                                  │ plan argument, else the ticket's plan field,
                                                                  │ else plan_path(<id>)
                                                                  └─ refuses while that file is missing
```

`cargo xtask ticket check` then holds the gate for every ticket: a `ready`, `running` or `review`
work ticket must name a plan, and every plan a ticket names must exist unless the ticket is an
`idea` or `cancelled`. The gates check that the plan exists, not what its sections say. A plan is live while its ticket is `idea`, `queued` or `ready`, and a frozen
record, never reworded, once the ticket ships or is cancelled.

## Code

- [Ticket engine](/tools_v2/ticket-engine/) — `PLANS_DIR`, `PLAN_TEMPLATE` and `plan_path` in
  `tools_v2/ticket-engine/src/repository.rs`; `mark_ready` in
  `tools_v2/ticket-engine/src/ops/readiness.rs`; the plan ready-gate in
  `tools_v2/ticket-engine/src/validation/readiness.rs`.
- [Ticketboard](/apps/ticketboard/) — shows a ticket's `plan` and opens it in the in-app document
  viewer with one click.

## Boundaries

- Depends on: the plan template `.ai/tickets/plan_template.md`; the `plan` field of the ticket
  files in `.ai/tickets/`.
- Used by: `cargo xtask ticket mark-ready`, `ticket check` and `ticket brief`, which prints the
  plan to read; the ticketboard; the ticket engine's tests, which build a plan under `PLANS_DIR`.
- Rules: one plan per ticket at its id-derived path unless the ticket's `plan` field names another;
  no subfolders; a work ticket goes ready only with its plan on disk (`cargo xtask ticket check`,
  `mark_ready`); a frozen plan is never reworded, and the gates judge it only on its links
  (`cargo xtask verify link-check`).

## Related documentation

- [Ticket specs and plans](/documentation_v2/tickets/README.md) — the lifecycle both folders share.
- [Ticket run pipeline](/documentation_v2/runbooks/ticket_run_pipeline.md) — writing the plan and
  marking a ticket ready.
- [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) — the id grammar behind
  the file names.
