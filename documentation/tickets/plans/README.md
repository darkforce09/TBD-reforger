**Status:** live

# Ticket plans

One plan per work [ticket](/documentation/glossary/n_to_z.md#ticket) that went ready: how the work
runs, what can go wrong and how it is verified. The agent working the ticket reads the plan beside
its spec; the spec says what to build, the plan how.

## Contents

```text
documentation/tickets/plans/
└── t-*_plan.md  one plan per ticket, named t-<id>_plan.md
```

## How it works

A plan's name is `t-`, its ticket's id lowercased with dots as underscores, then `_plan.md`: the
plan of ticket `T-<n>.<m>` is `t-<n>_<m>_plan.md`. The ticket's `plan` field in the legacy file
`.ai/tickets/T-<id>.toml` records that path, and `ttm import` reads the plan into the central
ticket manager.

A plan is a copy of `.ai/tickets/plan_template.md` with its four sections filled: Context,
Approach, Risks and Verification. Nothing goes ready without one:

```text
cp .ai/tickets/plan_template.md  ─▶  fill the four sections  ─▶  ttm --project reforger mark-ready <ticket>
                                                                  └─ refuses while the readiness gate does not hold
```

`ttm --project reforger check` then holds the gate for every ticket: a `ready`, `running` or `review`
work ticket must name a plan, and every plan a ticket names must exist unless the ticket is an
`idea` or `cancelled`. The gates check that the plan exists, not what its sections say. A plan is live while its ticket is `idea`, `queued` or `ready`, and a frozen
record, never reworded, once the ticket ships or is cancelled.

## Code

- [Ticket manager client](/tools/foundation/ticket_manager_client/README.md) — `brief`, which
  returns the plan to read from `ttm --project reforger brief`.

## Boundaries

- Depends on: the plan template `.ai/tickets/plan_template.md`; the `plan` field of the ticket
  files in `.ai/tickets/`.
- Used by: `ttm import`, which reads each plan into the central ticket manager, and
  `ttm --project reforger brief`, which prints the plan to read.
- Rules: one plan per ticket at its id-derived path unless the ticket's `plan` field names another;
  no subfolders; a work ticket goes ready only with its plan (`ttm --project reforger check`,
  `mark-ready`); a frozen plan is never reworded, and the gates judge it only on its links
  (`cargo xtask verify link-check`).

## Related documentation

- [Ticket specs and plans](/documentation/tickets/README.md) — the lifecycle both folders share.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — the run, land and
  close around a ready ticket.
- [Ticket identifiers](/documentation/standards/ticket_identifiers.md) — the id grammar behind
  the file names.
