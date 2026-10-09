# Ruler tool tests

Unit tests of the ruler: the polyline capture and its dismissal, the leg quantities and readouts,
the screen projection, the tool mode, and the guard that a ruler reading never writes the mission
document.

## Contents

```text
crates/mission_editing/map_editing_tools/src/ruler/tests/
├── chain.rs          tool-mode arbitration and the polyline capture: append, end, dedup, dismissal
├── leg.rs            the leg quantities and the bearing rule
└── projection.rs     label keying and the screen projection of a chain
```

## Boundaries

- Depends on: the ruler modules (each file mounted beside its module with a `#[path]` attribute)
  and the crate's `source_scrub`.
- Used by: `cargo test -p map_editing_tools`.
- Rules: pure arithmetic and formatting over in-memory chains; no asset, browser or GPU.
