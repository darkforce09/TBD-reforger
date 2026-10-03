**Status:** live

# Documentation move records

The first census and plan for moving the retired docs/ tree into `documentation_v2/`: the
disposition of every file it held and the cutover plan with the ticket citation rewrites. Status:
archived — frozen records.

## Contents

```text
documentation/archive/documentation_v2_refactor/
├── analysis_and_inventory.md  census of every docs/ file and its target, plus drift found later
└── architecture_plan.md       target layout, flat specs folder, citation rewrites, phased cutover
```

## Code

- [Ticket engine](/tools/tickets/ticket_model/) — `tools/ticket_engine/src/repository.rs`, where
  the ticket domain spells the documentation paths the plan moved.
- [xtask repository layout](/tools/foundation/repository_layout/) — the documentation path constants the
  documentation gates read.

## Boundaries

- Depends on: nothing live; the records quote the tree of their time.
- Used by: the [documentation program records](/documentation/archive/refactor_v2/README.md)
  (the move manifest and the program plan) and nothing else.
- Rules: never reworded, only links change.

## Related documentation

- [Documentation entry](/documentation/README.md) — the tree as it is.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the layout,
  lifecycle and gates the move produced.
- [Documentation program plan](/documentation/archive/refactor_v2/refactor_program_plan.md) — the program that
  carried the move out.
