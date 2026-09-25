# Browser rendering

The egui views of the [ticketboard](/documentation_v2/glossary.md#ticketboard)'s browser: the
filter bar, the status board and its cards, the program tree and the ticket detail panel. Each
paints from a `BrowserView` and reports what the viewer did as `BrowserEvent`s.

## Contents

```text
apps/ticketboard/src/ticket_browser/ui/
├── appearance.rs    the quarantine fence and `main_goal` tints
├── detail_panel/    one ticket in full: header, actions, comparison, metadata, body, links
├── filter_bar.rs    `filter_bar_ui`: text, executor, kind, scope, class, status, parent
├── mod.rs           the module tree, the callback types, status and class colours
├── program_tree.rs  `tree_ui`: the flattened tree as an indented virtualized list with toggles
├── status_board.rs  `board_ui`: a column per status, count chips, idea cards dragged to queued
└── ticket_card.rs   `card_ui`: one card painted into a single rect, with no nested widgets
```

## How it works

The application lends a `BrowserView` each frame and collects the events.

- `filter_bar_ui` edits the application's `Filters` in place and returns whether anything changed,
  so the application refilters once, outside the paint. Scope values the vocabulary does not know
  read "(not in vocab)"; "clear" is enabled while any filter is active.
- `board_ui` lays the eight columns side by side, 236 points wide, each a virtualized list of
  64-point cards drawn from the column's visible rows. `shipped` and `cancelled` start collapsed as
  a count chip that expands on click, and a "−" button collapses them again. A click selects a
  card, a shift-click picks it for comparison, and a right-click opens the ticket menu the
  application supplies. While no command runs, an `idea` card can be dragged onto the `queued`
  column, which emits `OpenAnchorDialog` for the same anchor picker as "Queue after…".
- `card_ui` paints the id, the `#order`, the title, the scope breadcrumb (with `~` and a tooltip
  when the scope is inferred from `owns`), the executor chip and the class chip, and shows the
  `main_goal` on hover, all with the painter alone so a column stays within the frame budget.
- `tree_ui` paints the flattened rows 18 points high, indented 16 points per level and coloured by
  status, dimming the ancestors a filter keeps only for context.
- `detail_panel/` paints the selected ticket.

`TicketMenu` and `TicketActionStrip` are the callback types through which the application lends
the menus of `crate::ticket_actions`; the browser wraps their events in `BrowserEvent::TicketAction`
and never imports that feature's `ui`.

## Public surface

- `status_board::board_ui`, `program_tree::tree_ui` and `detail_panel::metadata::detail_ui`: painted
  by `crate::application::feature_views` for the Board and Tree tabs and the detail column.
- `filter_bar::filter_bar_ui`: painted by `crate::application` above every tab once the corpus
  has loaded.

## Boundaries

- Depends on: `crate::ticket_browser::models` and `crate::ticket_browser::services`;
  `crate::ticket_registry::models` (`palette`, `projection`); `crate::ticket_actions::events` for
  the callback types; `crate::execution_metrics::estimated`, `crate::document_viewer::services`
  and `crate::wave_plan::services` in `detail_panel/`; `crate::core::ui`; `ticket_engine`
  (`StatusName`); `eframe::egui` and `egui_extras`.
- Used by: `crate::application` (`feature_views.rs` and `mod.rs`).
- Rules: rendering emits events and mutates only the `Filters` it is handed; it imports no other
  feature's `ui`, `crate::core::ui` excepted (the test
  `dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`); a dragged card opens a dialog, never a
  command.
