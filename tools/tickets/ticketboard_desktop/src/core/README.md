# Ticketboard core

The interface primitives the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) features
share, which know nothing about [tickets](/documentation/glossary/n_to_z.md#ticket).

## Contents

```text
tools/tickets/ticketboard_desktop/src/core/
├── mod.rs     the module tree: `ui`
└── ui/        the shared accent colours, the output row height and the identifier link
```

## How it works

`ui` holds the shared accent colours, the output row height and the identifier link every feature's
views reuse. The ticket-free subprocess, log and clock helpers are in
`ticketboard_model::core` (`tools/tickets/ticketboard_model/src/core/`).

## Public surface

- `ui`: the colours, `OUTPUT_ROW_H` and `identifier_link`, used by the `ui` modules of every
  feature.

## Boundaries

- Depends on: `eframe::egui`.
- Used by: the `ui` modules of every feature and `crate::application`.
- Rules: core imports no feature module and nothing from `ticket_model`, and a feature may import
  `core::ui` although it never imports another feature's `ui` (the test
  `dependency_boundaries_and_external_test_placement_are_enforced` in
  `tools/tickets/ticketboard_desktop/src/tests/architecture_rules.rs`).
