**Status:** live

# Tooling restructuring records

The records of the program that gathered every developer tool into the four crates and the npm
package under `tools_v2/`: its inventory, its target architecture and one record per phase with what
landed and the measurements taken. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/tools_v2_refactor/
├── analysis_and_inventory.md  what lived under tools_v2/, module by module, with each part's role
├── architecture_plan.md       the four crates, the invariants between them and their structural tests
└── phase_*_handoff.md         one record per phase, one to five: what landed and what was measured
```

## How it works

Read the architecture plan first, then the phase records in order: one, relocation of the crates;
two, the heavy services into `developer-tools`; three, the
[ticket](/documentation_v2/glossary/n_to_z.md#ticket) subsystem into `ticket-engine`; four, the split of
the two large crates by responsibility; five, the closing of the tree, the longest record. Paths,
commands and counts in them are as they stood when each was written.

## Code

- [Tooling](/tools_v2/) — the crates and package the program arranged.

## Boundaries

- Depends on: nothing live; the records quote the tooling of their time.
- Used by: the [tooling documentation](/documentation_v2/tools_v2/README.md), which links the
  archived architecture plan; the documentation program's own records.
- Rules: never reworded, only links change; a rule still in force lives in the tooling architecture
  document and the crates' tests, not here.

## Related documentation

- [Tooling documentation](/documentation_v2/tools_v2/README.md) and [tooling
  architecture](/documentation_v2/tools_v2/tooling_architecture.md) — the tools as they are.
- [Tooling README](/tools_v2/README.md) — the tooling tree.
