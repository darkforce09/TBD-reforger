# Execution metrics

The [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) feature that shows what running
[tickets](/documentation_v2/glossary/n_to_z.md#ticket) cost: the measured run receipts in
`.ai/tickets/metrics/<id>/` and the historical token estimates in `.ai/tickets/estimates/`, loaded,
checked, summed and drawn as two datasets that never share a total.

## Contents

```text
apps/ticketboard/src/execution_metrics/
├── estimated/  token estimates: the file check, per-class and per-domain sums, the ticket-detail cells
├── events.rs   `MetricsEvent`: select a ticket, or sort a measured or an estimated table
├── measured/   run receipts: the receipt check, per-ticket and per-agent sums, number formatting
├── mod.rs      the module tree
├── models/     `MetricsView`, the borrowed view the Metrics tab paints from
└── ui/         the Metrics tab: the measured panel above the estimated panel
```

## How it works

The application's loading thread reads both trees beside the corpus: `measured::load_metrics`
returns a finished `MetricsState`, and `estimated::load_raw` returns the checked estimate files,
which `estimated::build_state` joins to the corpus when the board is built. Both folders sit inside
the watched `.ai/tickets/` tree, so the debounced file watch reloads them with the corpus.

```text
measured:   .ai/tickets/metrics/<id>/*.json ──load_metrics──▶ MetricsState
estimated:  .ai/tickets/estimates/<id>.json ──load_raw──▶ RawEstimates
                                   + corpus ──build_state──▶ EstimatesState

MetricsState + EstimatesState ──MetricsView──▶ ui::dashboard (the Metrics tab)
EstimatesState ──stamp_cell, tokens_cell──▶ the ticket details
```

Each file is either a checked record in the sums or a named error row with its reason verbatim;
there is no third outcome, and an absent or empty folder is an explicit empty state instead of a
table of zeros. The checks are hand-kept copies of the engine's `validate_record` and
`validate_estimate` plus the schema patterns. Measured and estimated figures have distinct row,
total and model types, and estimated tokens are the `EstimatedTokens` type, so the two cannot be
added by accident. The tab's clicks come back as `MetricsEvent`s: sorting changes one table's
order, and a ticket link selects that ticket on the board.

## Public surface

- `measured`: `load_metrics`, `sort_rows` and the state and sort types, which `crate::application`
  loads, holds and sorts.
- `estimated`: `load_raw`, `build_state` and `sort_rows` for `crate::application`; `stamp_cell`,
  `tokens_cell`, `EstimatesState`, `ESTIMATE_GLYPH` and `ABSENT_ESTIMATED_MARKER` for the ticket
  details in `crate::ticket_browser`.
- `models::MetricsView`, `ui::dashboard::metrics_ui` and `events::MetricsEvent`: the tab, which
  `crate::application` lends the view, paints and turns into actions.

## Boundaries

- Depends on: `crate::ticket_registry::models` (the corpus and a ticket's class); `crate::core::ui`
  (the verdict and estimate colours and `identifier_link`); `ticket_engine` (`Ticket`,
  `repository::METRICS_DIR` and `repository::ESTIMATES_DIR`, `validate_rfc3339_utc`); `serde`,
  `serde_json` and `time`; `eframe::egui` and `egui_extras` in `ui/` only.
- Used by: `crate::application` (`background_loading.rs`, `workspace_state.rs`,
  `action_dispatch.rs`, `events.rs`, `feature_views.rs` and `mod.rs` in
  `apps/ticketboard/src/application/`); `crate::ticket_browser`, through `estimated`
  (`apps/ticketboard/src/ticket_browser/models/view.rs` and the detail panel in
  `apps/ticketboard/src/ticket_browser/ui/detail_panel/`).
- Rules:
  - no code path combines a measured and an estimated figure
    (`the_law_no_code_path_combines_measured_and_estimated` in
    `apps/ticketboard/src/execution_metrics/estimated/tests/estimated.rs`);
  - a malformed receipt or estimate is listed by name and left out of every sum (the tests in
    `measured/tests/measured.rs` and `estimated/tests/estimated.rs`);
  - `measured/`, `estimated/` and `models/` name no egui type, and no other feature imports `ui/`
    (`dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`);
  - the feature reads the two trees and never writes them.

## Related documentation

- [Token estimate factor](/documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md) — how
  the estimates are made.
