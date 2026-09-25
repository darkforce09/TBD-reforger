**Status:** live

# Contracts relocation records

The records of the move of the shared schemas into `contracts_v2/`: the census of every contract
file, the code generation and schema evolution policy of that time, and the handoff that maps each
old path to its new one. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/contracts_v2_relocation/
├── analysis_and_inventory.md  census of every contract file: what it governs and what reads it
├── architecture_plan.md       how Rust types are generated, how a schema may change, what CI checks
└── migration_handoff.md       where each contract file went; no schema changed in the move
```

## Code

- [Contracts](/contracts_v2/) — the tree the move created: definitions, rules, catalogs and
  fixtures.
- [Schema commands](/tools_v2/xtask/src/commands/schema/) — the code generation and checks the
  plan describes.

## Boundaries

- Depends on: nothing live; the records quote the tree of their time.
- Used by: the [contracts documentation](/documentation_v2/contracts_v2/README.md), which links the
  archived pipeline plan; the documentation program's own records.
- Rules: never reworded, only links change.

## Related documentation

- [Contracts documentation](/documentation_v2/contracts_v2/README.md) — the live description of the
  contracts.
- [Schema evolution policy](/documentation_v2/contracts_v2/schema_evolution_policy.md) — how a
  schema may change now.
