**Status:** live

# Engine split records

The records of the program that split the website's rendering and map code into the graphics
engine, the map engine and the browser app: the program with its target layout and phases, a
file-by-file blueprint of the crates, and the measured baseline before its third phase. Status:
archived — frozen records.

## Contents

```text
documentation_v2/archive/engine_split/
├── engine_split_phase3_baseline.md                gates, tests and frame cost measured before phase 3
├── engine_split_program.md                        target layout, three phases, gate and execution rules
└── graphics_engine_and_mission_core_blueprint.md  file-by-file blueprint of the crates and their manifests
```

## Code

- [Graphics engine](/apps/website/graphics-engine/) and [map engine](/apps/website/map-engine/) —
  the crates the split produced.
- [Frontend](/apps/website/frontend/) — the browser app the editing logic left.
- [Engine layer gate](/tools_v2/xtask/src/verifications/architecture/) —
  `cargo xtask verify engine-layers`, which holds the rules the program set.

## Boundaries

- Depends on: nothing live; the records quote the code of their time.
- Used by: the [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md), which
  link the program as the history of its rules; the documentation program's own records.
- Rules: never reworded, only links change; a rule still in force lives in the engine boundary
  rules, not here.

## Related documentation

- [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) — the live layer
  rules and the gate that holds them.
- [Map engine documentation](/documentation_v2/website/map-engine/README.md) and
  [graphics engine documentation](/documentation_v2/website/graphics-engine/README.md) — the crates
  as they are.
