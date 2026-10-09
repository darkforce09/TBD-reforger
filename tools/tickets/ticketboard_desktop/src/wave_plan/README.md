# Wave plan views

The egui views of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s wave plan
feature: the Waves tab: the recorded lanes, wave 0 and the pack-last tickets. The feature's models,
services and events are in
[`tools/tickets/ticketboard_model/src/wave_plan/`](/tools/tickets/ticketboard_model/src/wave_plan/README.md);
these views paint the borrowed view the application lends them and return the feature's events.

## Contents

```text
tools/tickets/ticketboard_desktop/src/wave_plan/
├── mod.rs     the module tree
└── ui/        the views
```

## Boundaries

- Depends on: `ticketboard_model::wave_plan` (its models and events) and the shared
  `ticketboard_model` models the views read; `crate::core::ui`; `eframe::egui`.
- Used by: `crate::application`, which lends the views their data and applies the events they
  return.
- Rules: the folder holds only views
  (`module_roots_and_documentation_describe_the_entire_source_tree` in
  `tools/tickets/ticketboard_desktop/src/tests/architecture_rules.rs`), and no other feature imports its `ui`
  (`dependency_boundaries_and_external_test_placement_are_enforced`).
