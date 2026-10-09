# Execution metrics

The [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) feature that shows what running
[tickets](/documentation/glossary/n_to_z.md#ticket) cost: the measured run receipts in
`.ai/tickets/metrics/<id>/` and the historical token estimates in `.ai/tickets/estimates/`, loaded,
checked and summed as two datasets that never share a total. The desktop application draws them
from `tools/tickets/ticketboard_desktop/src/execution_metrics/ui/`.

## Contents

```text
tools/tickets/ticketboard_model/src/execution_metrics/
├── estimated/  token estimates: the file check, per-class and per-domain sums, the ticket-detail cells
├── events.rs   `MetricsEvent`: select a ticket, or sort a measured or an estimated table
├── measured/   run receipts: the receipt check, per-ticket and per-agent sums, number formatting
├── mod.rs      the module tree
└── models/     `MetricsView`, the borrowed view the Metrics tab paints from
```

## How it works

The loading thread (`crate::application_state::background_loading`) reads both trees beside the corpus: `measured::load_metrics`
returns a finished `MetricsState`, and `estimated::load_raw` returns the checked estimate files,
which `estimated::build_state` joins to the corpus when the board is built. Both folders sit inside
the watched `.ai/tickets/` tree, so the debounced file watch reloads them with the corpus.

```text
measured:   .ai/tickets/metrics/<id>/*.json ──load_metrics──▶ MetricsState
estimated:  .ai/tickets/estimates/<id>.json ──load_raw──▶ RawEstimates
                                   + corpus ──build_state──▶ EstimatesState

MetricsState + EstimatesState ──MetricsView──▶ the application's Metrics tab
EstimatesState ──stamp_cell, tokens_cell──▶ the ticket details
```

Each file is either a checked record in the sums or a named error row with its reason verbatim;
there is no third outcome, and an absent or empty folder is an explicit empty state instead of a
table of zeros. The checks are semantic mirrors of `ticket_metrics::validate_record` and
`ticket_metrics::estimates::validate_estimate` plus the schema patterns, reading the
`ticket_metrics` record shapes. Measured and estimated figures have distinct row,
total and model types, and estimated tokens are the `EstimatedTokens` type, so the two cannot be
added by accident. The tab's clicks come back as `MetricsEvent`s: sorting changes one table's
order, and a ticket link selects that ticket on the board.

## Public surface

- `measured`: `load_metrics`, `sort_rows` and the state and sort types, which
  `crate::application_state` loads and holds and the desktop application sorts.
- `estimated`: `load_raw`, `build_state` and `sort_rows` for the same owners; `stamp_cell`,
  `tokens_cell`, `EstimatesState`, `ESTIMATE_GLYPH` and `ABSENT_ESTIMATED_MARKER` for the ticket
  details the application paints.
- `models::MetricsView` and `events::MetricsEvent`: the tab's borrowed view and its clicks, which
  the application lends and paints, and `crate::application_state::events` turns into actions.

## Boundaries

- Depends on: `crate::ticket_registry::models` (the corpus and a ticket's class); `ticket_model`
  (`Ticket`); `ticket_metrics` (`RunRecord`, `CohortKey`); `repository_layout` (`METRICS_DIR` and
  `ESTIMATES_DIR`); `time_source` (`validate_rfc3339_utc`); `serde`, `serde_json` and `time`.
- Used by: `crate::application_state` (`background_loading.rs`, `workspace_state.rs`,
  `events.rs`); `crate::ticket_browser`, through `estimated`
  (`tools/tickets/ticketboard_model/src/ticket_browser/models/view.rs`); the desktop application:
  `tools/tickets/ticketboard_desktop/src/execution_metrics/ui/`, the detail panel in
  `tools/tickets/ticketboard_desktop/src/ticket_browser/ui/detail_panel/`, and `tools/tickets/ticketboard_desktop/src/application/`
  (`action_dispatch.rs`, `feature_views.rs`, `mod.rs`).
- Rules:
  - no code path combines a measured and an estimated figure;
  - a malformed receipt or estimate is listed by name and left out of every sum;
  - nothing here names egui;
  - the feature reads the two trees and never writes them.

## Related documentation

- [Token estimate factor](/documentation/tools/tickets/token_estimate_factor.md) — how
  the estimates are made.
