# Ticket actions views

The egui views of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s ticket actions
feature: the card menu and the detail panel's action strip, the mutation dialogs, and the command
chip, drawer and toasts. The feature's models, services and events are in
[`tools/tickets/ticketboard_model/src/ticket_actions/`](/tools/tickets/ticketboard_model/src/ticket_actions/README.md);
these views paint the borrowed view the application lends them and return the feature's events.

## Contents

```text
tools/tickets/ticketboard_desktop/src/ticket_actions/
├── mod.rs     the module tree
└── ui/        the views
```

## Boundaries

- Depends on: `ticketboard_model::ticket_actions` (its models and events) and the shared
  `ticketboard_model` models the views read; `crate::core::ui`; `eframe::egui`.
- Used by: `crate::application`, which lends the views their data and applies the events they
  return.
- Rules: the folder holds only views
  (`module_roots_and_documentation_describe_the_entire_source_tree` in
  `tools/tickets/ticketboard_desktop/src/tests/architecture_rules.rs`), and no other feature imports its `ui`
  (`dependency_boundaries_and_external_test_placement_are_enforced`).
