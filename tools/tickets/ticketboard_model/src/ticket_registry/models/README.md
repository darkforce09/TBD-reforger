# Ticket registry models

The [ticket](/documentation/glossary/n_to_z.md#ticket) data every
[ticketboard](/documentation/glossary/n_to_z.md#ticketboard) feature reads: the loaded corpus and its
refusal, and the shared projections of a ticket (field access over both ticket kinds, status
columns, id order, class, scope breadcrumb and accent colours).

## Contents

```text
tools/tickets/ticketboard_model/src/ticket_registry/models/
├── classification.rs  `Class`: the five work-ticket classes and their accent colours
├── corpus.rs          `Corpus`, `LoadedTicket`, `Counts`, `LoadError` and `LoadResult`
├── mod.rs             the module tree
├── palette.rs         `status_rgb` and `scope_level_rgb`, the status and breadcrumb accents
├── projection.rs      `TicketView` and `view`, the status column order, `id_sort_key`, field helpers
└── scope.rs           `Breadcrumb` and `breadcrumb`: a work ticket's scope, domain to surface
```

## How it works

A `Corpus` holds every parsed `ticket_model::Ticket` with the file it came from, and `Counts` whose
parents and children add up to the total. A `LoadError` names the one file that refused the load
and carries its error verbatim.

`projection.rs` is the one place features read ticket fields. `view(ticket)` returns a
`TicketView` over both kinds, program and work, with the fields a kind lacks empty; `title_of`,
`class_of`, `shipped_at_of`, `status_label` and the other helpers read single fields.
`STATUS_ORDER` sets the eight board columns in the declaration order of `StatusName`, and
`collapsed_by_default` starts `shipped` and `cancelled` collapsed. `id_sort_key` orders ids
numerically, each dotted segment compared as a number rather than as text. An absent `executor`
reads as `EXECUTOR_DEFAULT`, `claude-code`. The module re-exports `classification` and `scope`.

`breadcrumb` joins a work ticket's scope with ` › `, ending at the layer when the scope has no
component; a component with no surface is marked "(no surface)" in the ticket details, and a scope
listed in the ticket's `estimated` field carries `SCOPE_ESTIMATED_GLYPH` (`~`). Colour is an accent
only: the raw lowercase status and class names stay the labels.

## Boundaries

- Depends on: `ticket_model` (`Ticket`, `Status`, `StatusName`, `ScopeV2`).
- Used by: every other feature: `crate::ticket_browser` (the board, program tree, filters and
  detail sections), `crate::ticket_actions` (models, commands and dialog builders),
  `crate::wave_plan` (the wave projection), `crate::execution_metrics::estimated` (the corpus and
  a ticket's class); `crate::application_state` (`Corpus`, `LoadError` and the projection);
  `tools/tickets/ticketboard_model/src/tests/support/mod.rs`; the desktop application, whose
  `ui/` folders read the projection, the palettes and the classes for cards, details, menus,
  dialogs and lane colours.
- Rules: `Class::ALL` equals `ticket_model::CLASS_VALUES` (a test in
  `tools/tickets/ticketboard_model/src/ticket_browser/models/tests/status_board.rs`); `column_of`,
  `Class` and the palettes match every variant with no wildcard, so a new status or class fails to compile until it
  has a column and a colour; no egui type appears here
  (`model_dependency_boundaries_and_external_test_placement_are_enforced` in
  `tools/tickets/ticketboard_model/src/tests/architecture_rules.rs`).
