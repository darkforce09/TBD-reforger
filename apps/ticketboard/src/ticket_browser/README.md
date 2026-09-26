# Ticket browser

The [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) feature that presents the loaded
[ticket](/documentation_v2/glossary/n_to_z.md#ticket) registry: the status board, the program tree, the
filters, a ticket's full details, and the ownership comparison of two tickets. It reads the
registry and never changes it.

## Contents

```text
apps/ticketboard/src/ticket_browser/
├── events.rs  `BrowserEvent`: select, compare, toggle, open a path or document, copy, ticket actions
├── mod.rs     the module tree
├── models/    board columns and cards, the program tree, detail-section order, `BrowserView`
├── services/  per-ticket filter facts, composable filters, and the scope facets from the vocabulary
└── ui/        the filter bar, status board, cards, program tree and detail panel
```

## How it works

The projections are built once per load, and the filters once per change, so painting a frame
only reads:

```text
Corpus ──load──▶ BoardModel, TreeModel, FilterIndex  (models/, services/)
Filters change ──refilter──▶ facet options, per-ticket verdicts, visible rows, tree rows
BrowserView (borrowed) ──▶ ui/ ──▶ BrowserEvent ──▶ application actions
```

`WorkspaceState` in `apps/ticketboard/src/application/workspace_state.rs` owns the built models
and the filters; its `refilter` runs `scope_facets::compute` and `Filters::apply`, then derives
each column's visible cards and the flattened tree from the verdicts. Each frame the application
lends a `BrowserView` to the Board and Tree tabs and the detail column, and turns the returned
`BrowserEvent`s into its own actions.

Ticket actions reach the browser only as callbacks: the application passes the card menu and the
detail action strip of `crate::ticket_actions` as `TicketMenu` and `TicketActionStrip`, and their
events come back wrapped in `BrowserEvent::TicketAction`. Dropping an `idea` card on the `queued`
column emits `OpenAnchorDialog`, which the application answers with the anchor picker.

## Public surface

- `models`: `BoardModel`, `TreeModel` with `flatten`, and `BrowserView`, which the application
  builds, keeps and lends.
- `services`: `FilterIndex`, `Filters`, `VocabTree`, `FacetOptions` and `compute`, which the
  application loads and refilters with.
- `ui`: `filter_bar::filter_bar_ui`, `status_board::board_ui`, `program_tree::tree_ui` and
  `detail_panel::metadata::detail_ui`, painted by the application.
- `events::BrowserEvent`, which the application converts into its actions.

## Boundaries

- Depends on: `crate::ticket_registry::models` (`Corpus`, `projection`, `palette`);
  `crate::ticket_actions::events` for the forwarded events; `crate::execution_metrics::estimated`
  for the estimated stamps and tokens; `crate::document_viewer::services::document_loading` to
  tell a document link from a file link; `crate::wave_plan::services::lock_file` for the ownership
  collision rule; `crate::core::ui`; `ticket_engine` (`StatusName`, `Ticket`,
  `repository::SCOPE_VOCAB`); `toml`; `eframe::egui` and `egui_extras` in `ui/` only.
- Used by: `crate::application` (`mod.rs`, `events.rs`, `feature_views.rs`, `workspace_state.rs`,
  `background_loading.rs`, `window.rs`).
- Rules:
  - the browser may use other features' models, services and events, never their `ui` or
    application state, and `models/` and `services/` never name egui (the test
    `dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`);
  - filters change projections once per change, never per frame, and never the registry
    (`filters_compose_as_intersection` and `clear_restores_the_full_measured_count` in
    `services/tests/filtering.rs`).

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, fields and statuses the browser
  shows.
