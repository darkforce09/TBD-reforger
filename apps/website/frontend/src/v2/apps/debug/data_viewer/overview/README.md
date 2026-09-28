# Equipment data viewer overview

The viewer's summary tabs: the selected generation's counts and capability entry points, the
gameplay selection decisions, and the export history.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/data_viewer/overview/
├── mod.rs                `Overview` and `GenerationHistory`: counts, entry points, published generations
└── selection_summary.rs  `SelectionSummary`: every reviewed field's decision, searchable and paged
```

## Boundaries

- Depends on: the viewer's location and requests in
  `apps/website/frontend/src/v2/apps/debug/data_viewer/`, its layout components, and the dataset
  DTOs in `apps/website/frontend/src/v2/core/api/dto/equipment_data_viewer/`.
- Used by: `page.rs` in `apps/website/frontend/src/v2/apps/debug/data_viewer/`, for the
  `overview`, `selection` and `generations` tabs.
- Rules: every count comes from the selected generation's answer; the selection tab shows the
  decision kinds, the diagnostic-only dependencies left out and each decision's native type,
  disposition, section and reason as recorded.
