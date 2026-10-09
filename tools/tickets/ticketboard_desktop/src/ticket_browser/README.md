# Ticket browser views

The egui views of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s ticket browser
feature: the filter bar, the status board and its cards, the program tree, and the ticket details
with the two-ticket comparison. The feature's models, services and events are in
[`tools/tickets/ticketboard_model/src/ticket_browser/`](/tools/tickets/ticketboard_model/src/ticket_browser/README.md);
these views paint the borrowed view the application lends them and return the feature's events.

## Contents

```text
tools/tickets/ticketboard_desktop/src/ticket_browser/
├── mod.rs     the module tree
└── ui/        the views
```

## Boundaries

- Depends on: `ticketboard_model::ticket_browser` (its models and events) and the shared
  `ticketboard_model` models the views read; `crate::core::ui`; `eframe::egui`.
- Used by: `crate::application`, which lends the views their data and applies the events they
  return.
- Rules: the folder holds only views, and no other feature imports its `ui`.
