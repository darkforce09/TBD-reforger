**Status:** live

# Shipped history

The log of what the platform and the [mod](/documentation_v2/glossary.md#mod) shipped, program by
program and [ticket](/documentation_v2/glossary.md#ticket) by ticket, kept out of the agent
instruction file so agents do not load it as working context. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/shipped_history/
└── shipped_history.md  shipped slices by program: what landed, where and at which commit
```

## Code

None: the log records shipped tickets, not a code folder.

## Boundaries

- Depends on: nothing live; the log quotes the code and tickets of its time.
- Used by: the ticket files in `.ai/tickets/` whose citations name a section of the log as their
  source; `ARCHIVED_WAVE_PLAN_READERS` in `tools_v2/ticket-engine/src/repository.rs`, which lets the
  log name the archived wave plans.
- Rules: never reworded, only links change; the log is not working context, so `CLAUDE.md` and the
  agent rules do not point agents at it; what shipped is recorded by each ticket's `shipped_at`
  commit and the git history.

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — every ticket's status and landing commit.
- [Product roadmap](/documentation_v2/product_roadmap.md) — what is planned next.
