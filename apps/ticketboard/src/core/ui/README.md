# Shared interface primitives

The small egui pieces every [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) feature
draws with: the accent colours shared across tabs, the height of an output row, and the
identifier link.

## Contents

```text
apps/ticketboard/src/core/ui/
└── mod.rs  the accent colours, `OUTPUT_ROW_H` and `identifier_link`
```

## How it works

`VERDICT_OK` and `VERDICT_COLLIDE` colour a clean or colliding ownership verdict, and the mutation
dialogs reuse the same pair. `SCOPE_ESTIMATED_COLOR` marks every estimated figure (the `~` stamp
glyph, the estimated tokens row, the estimated panel header), so estimates read the same on every
surface. `identifier_link` renders a ticket id as a monospace link only when its target exists,
and as plain monospace text otherwise.

## Boundaries

- Depends on: `eframe::egui` (`Color32`, `RichText`, `Ui`).
- Used by: the `ui` modules of `crate::document_viewer`, `crate::execution_metrics`,
  `crate::repository_status`, `crate::ticket_actions` and `crate::ticket_browser`.
- Rules: this is the only rendering code one feature may import from outside itself; the
  architecture test `dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs` refuses every other cross-feature UI import.
