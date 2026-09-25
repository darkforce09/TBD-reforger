# Metrics tab

The egui drawing of the [ticketboard](/documentation_v2/glossary.md#ticketboard)'s Metrics tab: a
measured panel over the run receipts and, below a double rule, an estimated panel over the token
estimates, each with its own headline, tables, sorting and colour.

## Contents

```text
apps/ticketboard/src/execution_metrics/ui/
├── dashboard.rs         `metrics_ui`: both panels on one scroll surface, split by a double rule
├── estimated_tables.rs  the estimated panel: strip, per-class and per-domain tables, malformed files
├── measured_tables.rs   the measured panel: strip, coverage note, per-agent and per-ticket tables
└── mod.rs               the module tree, `panel_badge_ui` and `id_link_ui`
```

## How it works

`metrics_ui` draws the "MEASURED — run receipts" badge in the `VERDICT_OK` colour, then either the
"No receipts yet" state with `no_receipts_text` and the coverage note, or the headline strip, the
per-agent and per-ticket tables and the malformed receipts, each named with its reason. Under the
double rule, the "ESTIMATED (historical)" badge and `NEVER_COMBINED_NOTE` take
`SCOPE_ESTIMATED_COLOR`, followed by either `no_estimates_text` or the estimated strip, the
per-class and per-domain tables and the malformed estimates. Tables are `egui_extras` tables whose
headers are sort buttons; a click emits `MetricsEvent::SortMetrics` or
`MetricsEvent::SortEstimates`. A ticket id in the per-ticket table is a link only when the ticket
exists, and emits `MetricsEvent::SelectId`. An elapsed time or last finish that is unknown shows
"—", never zero.

## Boundaries

- Depends on: `crate::execution_metrics` (`models::MetricsView`, `events::MetricsEvent`, the
  `measured` and `estimated` types, texts and sorts); `crate::core::ui` (`VERDICT_OK`,
  `VERDICT_COLLIDE`, `SCOPE_ESTIMATED_COLOR`, `identifier_link`); `eframe::egui` and
  `egui_extras`.
- Used by: `metrics_ui` in `apps/ticketboard/src/application/feature_views.rs`, which calls
  `dashboard::metrics_ui` and turns the events into actions.
- Rules: the measured and estimated panels never share a table, a strip or a total; no other
  feature imports this module (`dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`).
