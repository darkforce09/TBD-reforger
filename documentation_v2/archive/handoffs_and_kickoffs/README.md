**Status:** live

# Agent handoffs and kickoffs

Documents written to start or hand over one agent session: the kickoff of a Mission Creator UI
session, the phased agent execution contract of the first
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) program, and a pointer that sent
[mod](/documentation_v2/glossary/g_to_m.md#mod) agents to the monorepo ticketing. Status: archived —
frozen records.

## Contents

```text
documentation_v2/archive/handoffs_and_kickoffs/
├── editor_ui_handoff.md                kickoff of a session on the Mission Creator's UI and UX
├── mission_creator_agent_execution.md  phased agent contract of the first Mission Creator program
└── mod_claude_continuation.md          pointer from the mod agent handoff to the monorepo tickets
```

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the editor the first two
  documents plan work on.
- [Mod](/apps/mod/) — the addons the continuation pointer concerns.

## Boundaries

- Depends on: nothing live; the documents quote the code, queues and tickets of their time.
- Used by: the [mod documentation](/documentation_v2/mod/README.md), which links the mod
  continuation pointer; the documentation program's own records.
- Rules: never reworded, only links change; the execution contract's decisions live in the Mission
  Creator decisions log, not here.

## Related documentation

- [Mission Creator decisions](/documentation_v2/website/frontend/apps/editor/decisions.md) — the
  decisions the execution contract recorded, as live entries.
- [Ticket run pipeline](/documentation_v2/runbooks/ticket_run_pipeline.md) and
  [mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how agent work starts
  now.
