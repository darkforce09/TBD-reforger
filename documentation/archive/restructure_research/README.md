**Status:** live

# Restructure research

The explorer, planning and verification reports that the workspace restructure program rests on,
written on 2026-10-01 before any code moved. Status: archived — frozen records; they quote the
paths and counts of their time.

## Contents

```text
documentation/archive/restructure_research/
├── 00_architecture_blueprint_draft.md  the operator's first draft of the crate workspace, which the program corrects
├── 01_map_engine_graph.md              map engine module graph, feature gates, consumers, sizes
├── 02_path_coupling.md                 every hard-coded path, gate pins, tickets and runtime paths
├── 03_improved_layout_claims.md        what held and what failed in the improved layout plans
├── 04_first_plan_design.md             first planning pass: corrected crate topology and stages
├── 05_api_frontend_split.md            API domain cycles, application state, frontend inversions
├── 06_tools_split_and_claims.md        tooling split, duplicated helpers, dependency drift, claim checks
├── 07_engine_fine_grained.md           fine-grained engine crates, render engine field access, duplicates
├── 08_plan_draft_v1.md                 the first plan, rejected as too coarse
├── 09_design_v2.md                     the foundations-up design the program adopted
└── 10_verification_of_v2.md            adversarial verification of that design and its corrections
```

## How it works

The draft that started the program is 00; the reports are numbered in the order they were
written. The adopted design is report 09 as corrected by report 10; the live program documents
carry the result. A report is never edited after it lands.

## Code

- [Map engine](/apps/website/map-engine/), [API](/apps/website/api_v2/),
  [frontend](/apps/website/frontend/) and [tooling](/tools/) — the code the reports measured.

## Boundaries

- Depends on: nothing live; the reports quote the code of their date.
- Used by: the [restructure program](/documentation/restructure/README.md), whose findings cite
  these reports.
- Rules: never reworded; only links change.

## Related documentation

- [Restructure program](/documentation/restructure/README.md) — the live plan, target tree and
  progress built from these reports.
