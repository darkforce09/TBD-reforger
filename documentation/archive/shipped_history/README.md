**Status:** live

# Shipped history

The log of what the platform and the [mod](/documentation/glossary/g_to_m.md#mod) shipped, program by
program and [ticket](/documentation/glossary/n_to_z.md#ticket) by ticket, kept out of the agent
instruction file so agents do not load it as working context. Status: archived — frozen records.

## Contents

```text
documentation/archive/shipped_history/
└── shipped_history.md  shipped slices by program: what landed, where and at which commit
```

## Code

None: the log records shipped tickets, not a code folder.

## Boundaries

- Depends on: nothing live; the log quotes the code and tickets of its time.
- Used by: the ticket files in `.ai/tickets/` whose citations name a section of the log as their
  source; `ARCHIVED_WAVE_PLAN_READERS` in `tools/ticket_engine/src/repository.rs`, which lets the
  log name the archived wave plans.
- Rules: never reworded, only links change; the log is not working context, so `CLAUDE.md` and the
  agent rules do not point agents at it; what shipped is recorded by each ticket's `shipped_at`
  commit and the git history.

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — every ticket's status and landing commit.
- [Product roadmap](/documentation/product_roadmap.md) — what is planned next.
