# Metrics view

The borrowed view the [ticketboard](/documentation_v2/glossary.md#ticketboard)'s application lends
the Metrics tab for one frame.

## Contents

```text
apps/ticketboard/src/execution_metrics/models/
└── mod.rs  `MetricsView`: both states, both sort pairs and the ticket id index
```

## Boundaries

- Depends on: `crate::execution_metrics::measured` (`MetricsState`, `SortPair`) and
  `crate::execution_metrics::estimated` (`EstimatesState`, `EstimatedSortPair`).
- Used by: `metrics_ui` in `apps/ticketboard/src/application/feature_views.rs`, which builds the
  view from the loaded board; `crate::execution_metrics::ui`, which paints from it.
- Rules: the view only borrows, so painting cannot change the loaded data; the measured and
  estimated states stay separate fields with separate sorts; the ticket links read the board's
  id-to-index map instead of the browser's state.
