# Ticket browser

The models behind the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) feature that
presents the loaded [ticket](/documentation/glossary/n_to_z.md#ticket) registry: the status board,
the program tree, the filters and scope facets, and the order of a ticket's detail sections. The
desktop application paints them from `tools/tickets/ticketboard_desktop/src/ticket_browser/ui/`. The feature reads
the registry and never changes it.

## Contents

```text
tools/tickets/ticketboard_model/src/ticket_browser/
├── events.rs  `BrowserEvent`: select, compare, toggle, open a path or document, copy, ticket actions
├── mod.rs     the module tree
├── models/    board columns and cards, the program tree, detail-section order, `BrowserView`
└── services/  per-ticket filter facts, composable filters, and the scope facets from the vocabulary
```

## How it works

The projections are built once per load, and the filters once per change, so painting a frame
only reads:

```text
Corpus ──load──▶ BoardModel, TreeModel, FilterIndex  (models/, services/)
Filters change ──refilter──▶ facet options, per-ticket verdicts, visible rows, tree rows
BrowserView (borrowed) ──▶ application ui ──▶ BrowserEvent ──▶ Action
```

`WorkspaceState` in `tools/tickets/ticketboard_model/src/application_state/workspace_state.rs` owns the built models
and the filters; its `refilter` runs `scope_facets::compute` and `Filters::apply`, then derives
each column's visible cards and the flattened tree from the verdicts. Each frame the desktop
application lends a `BrowserView` to the Board and Tree tabs and the detail column it paints, and
`crate::application_state::events` turns the returned `BrowserEvent`s into `Action`s.

Ticket actions reach the browser only as events: the application's card menu and detail action
strip emit `crate::ticket_actions::events::TicketActionEvent`, which comes back wrapped in
`BrowserEvent::TicketAction`. Dropping an `idea` card on the `queued` column emits
`OpenAnchorDialog`, which the application answers with the anchor picker.

## Public surface

- `models`: `BoardModel`, `TreeModel` with `flatten`, the detail-section order and the triage
  block, and `BrowserView`, which `WorkspaceState` builds and keeps and the application lends.
- `services`: `FilterIndex`, `Filters`, `load_vocabulary`, `FacetOptions` and `compute`, which
  `crate::application_state` loads and refilters with.
- `events::BrowserEvent`, which `crate::application_state::events` converts into `Action`s.

## Boundaries

- Depends on: `crate::ticket_registry::models` (`Corpus`, `projection`);
  `crate::ticket_actions::events` for the forwarded events;
  `crate::execution_metrics::estimated::EstimatesState`, which `BrowserView` lends; `ticket_model`
  (`StatusName`, `Ticket`, `TicketId`, `ScopeVocab`).
- Used by: `crate::application_state` (`events.rs`, `workspace_state.rs`, `workspace_reload.rs`,
  `background_loading.rs`); the desktop application's `tools/tickets/ticketboard_desktop/src/ticket_browser/ui/`
  and `tools/tickets/ticketboard_desktop/src/application/` (`mod.rs`, `feature_views.rs`, `action_dispatch.rs`).
- Rules:
  - the browser may use other features' models, services and events, never application state,
    and nothing here names egui;
  - filters change projections once per change, never per frame, and never the registry.

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, fields and statuses the browser
  shows.
