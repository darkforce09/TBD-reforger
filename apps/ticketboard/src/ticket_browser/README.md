# `ticket_browser/`

## Responsibility

Presents status columns, the program tree, filters, ticket details, and ownership comparisons over the loaded registry.

## Public surface

`models::status_board::BoardModel` precomputes card labels and stable ordering. Filtering and facet services produce display projections. `BrowserView` supplies read-only UI data; `BrowserEvent` reports interactions. Rendering exposes menu and action-strip callbacks for application composition.

## Dependency rules

May consume other features' models, services, and event contracts, but never their UI or application state. The application supplies ticket-action render callbacks. Browser services and models remain independent of egui. Filter changes recompute projections once, not on every frame.

## Files

- [events.rs](events.rs) — Events.
- [mod.rs](mod.rs) — Module interface and composition.
- [models/detail_sections.rs](models/detail_sections.rs) — Detail sections.
- [models/mod.rs](models/mod.rs) — Module interface and composition.
- [models/program_tree.rs](models/program_tree.rs) — Program tree.
- [models/status_board.rs](models/status_board.rs) — Status board.
- [models/tests/detail_sections.rs](models/tests/detail_sections.rs) — Tests for detail sections.
- [models/tests/program_tree.rs](models/tests/program_tree.rs) — Tests for program tree.
- [models/tests/status_board.rs](models/tests/status_board.rs) — Tests for status board.
- [models/view.rs](models/view.rs) — View.
- [services/filtering.rs](services/filtering.rs) — Filtering.
- [services/mod.rs](services/mod.rs) — Module interface and composition.
- [services/scope_facets.rs](services/scope_facets.rs) — Scope facets.
- [services/tests/filtering.rs](services/tests/filtering.rs) — Tests for filtering.
- [services/tests/scope_facets.rs](services/tests/scope_facets.rs) — Tests for scope facets.
- [ui/appearance.rs](ui/appearance.rs) — Appearance.
- [ui/detail_panel/body_sections.rs](ui/detail_panel/body_sections.rs) — Body sections.
- [ui/detail_panel/cells.rs](ui/detail_panel/cells.rs) — Cells.
- [ui/detail_panel/comparison.rs](ui/detail_panel/comparison.rs) — Comparison.
- [ui/detail_panel/metadata.rs](ui/detail_panel/metadata.rs) — Metadata.
- [ui/detail_panel/mod.rs](ui/detail_panel/mod.rs) — Module interface and composition.
- [ui/detail_panel/reference_lists.rs](ui/detail_panel/reference_lists.rs) — Reference lists.
- [ui/filter_bar.rs](ui/filter_bar.rs) — Filter bar.
- [ui/mod.rs](ui/mod.rs) — Module interface and composition.
- [ui/program_tree.rs](ui/program_tree.rs) — Program tree.
- [ui/status_board.rs](ui/status_board.rs) — Status board.
- [ui/ticket_card.rs](ui/ticket_card.rs) — Ticket card.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
