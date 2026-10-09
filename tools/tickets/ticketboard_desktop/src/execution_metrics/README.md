# Execution metrics views

The egui views of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s execution
metrics feature: the Metrics tab: the measured and the estimated tables and strips. The feature's
models, services and events are in
[`tools/tickets/ticketboard_model/src/execution_metrics/`](/tools/tickets/ticketboard_model/src/execution_metrics/README.md);
these views paint the borrowed view the application lends them and return the feature's events.

## Contents

```text
tools/tickets/ticketboard_desktop/src/execution_metrics/
├── mod.rs     the module tree
└── ui/        the views
```

## Boundaries

- Depends on: `ticketboard_model::execution_metrics` (its models and events) and the shared
  `ticketboard_model` models the views read; `crate::core::ui`; `eframe::egui`.
- Used by: `crate::application`, which lends the views their data and applies the events they
  return.
- Rules: the folder holds only views, and no other feature imports its `ui`.
