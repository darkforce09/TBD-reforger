# `execution_metrics/`

## Responsibility

Loads, validates, aggregates, and displays measured run receipts and historical token estimates as separate datasets.

## Public surface

`measured` owns receipt schemas, aggregation, sorting, and display formatting. `estimated` owns estimate schemas, provenance cells, aggregation, and sorting. `MetricsView` supplies both datasets and a ticket identifier index. `MetricsEvent` reports independent sort changes and ticket selection.

## Dependency rules

Measured and estimated totals remain structurally separate. Shared formatting and validation do not combine either dataset. Model and service code imports no egui. The UI consumes an identifier index rather than browser state. Malformed observations retain named errors and are excluded from aggregates.

## Files

- [estimated/aggregation.rs](estimated/aggregation.rs) — Aggregation.
- [estimated/detail_projection.rs](estimated/detail_projection.rs) — Detail projection.
- [estimated/mod.rs](estimated/mod.rs) — Module interface and composition.
- [estimated/models.rs](estimated/models.rs) — Models.
- [estimated/services.rs](estimated/services.rs) — Services.
- [estimated/tests/estimated.rs](estimated/tests/estimated.rs) — Tests for estimated.
- [estimated/validation.rs](estimated/validation.rs) — Validation.
- [events.rs](events.rs) — Events.
- [measured/aggregation.rs](measured/aggregation.rs) — Aggregation.
- [measured/formatting.rs](measured/formatting.rs) — Formatting.
- [measured/mod.rs](measured/mod.rs) — Module interface and composition.
- [measured/models.rs](measured/models.rs) — Models.
- [measured/services.rs](measured/services.rs) — Services.
- [measured/tests/measured.rs](measured/tests/measured.rs) — Tests for measured.
- [mod.rs](mod.rs) — Module interface and composition.
- [models/mod.rs](models/mod.rs) — Module interface and composition.
- [ui/dashboard.rs](ui/dashboard.rs) — Dashboard.
- [ui/estimated_tables.rs](ui/estimated_tables.rs) — Estimated tables.
- [ui/measured_tables.rs](ui/measured_tables.rs) — Measured tables.
- [ui/mod.rs](ui/mod.rs) — Module interface and composition.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
