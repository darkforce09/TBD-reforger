**Status:** live

# Documentation move records

The first census and plan for moving the retired docs/ tree into `documentation_v2/`: the
disposition of every file it held and the cutover plan with the ticket citation rewrites. Status:
archived — frozen records.

## Contents

```text
documentation_v2/archive/documentation_v2_refactor/
├── analysis_and_inventory.md  census of every docs/ file and its target, plus drift found later
└── architecture_plan.md       target layout, flat specs folder, citation rewrites, phased cutover
```

## Code

- [Ticket engine](/tools_v2/ticket-engine/) — `tools_v2/ticket-engine/src/repository.rs`, where
  the ticket domain spells the documentation paths the plan moved.
- [xtask repository layout](/tools_v2/xtask/src/core/) — the documentation path constants the
  documentation gates read.

## Boundaries

- Depends on: nothing live; the records quote the tree of their time.
- Used by: the documentation program's own records at the documentation root (the move manifest
  and the program plan) and nothing else.
- Rules: never reworded, only links change; the program's live records (the plan, the writing
  brief, the checkpoint and the manifests) stay at the documentation root, not here.

## Related documentation

- [Documentation entry](/documentation_v2/README.md) — the tree as it is.
- [Documentation standards](/documentation_v2/standards/documentation_standards.md) — the layout,
  lifecycle and gates the move produced.
- [Documentation program plan](/documentation_v2/refactor_program_plan.md) — the program that
  carried the move out.
