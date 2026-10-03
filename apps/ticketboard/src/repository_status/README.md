# Repository status views

The egui views of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s repository
status feature: the status banner: the strict check, its output pane, the `git status` chip and the
watch notes. The feature's models, services and events are in
[`tools/tickets/ticketboard_model/src/repository_status/`](/tools/tickets/ticketboard_model/src/repository_status/README.md);
these views paint the borrowed view the application lends them and return the feature's events.

## Contents

```text
apps/ticketboard/src/repository_status/
├── mod.rs     the module tree
└── ui/        the views
```

## Boundaries

- Depends on: `ticketboard_model::repository_status` (its models and events) and the shared
  `ticketboard_model` models the views read; `crate::core::ui`; `eframe::egui`.
- Used by: `crate::application`, which lends the views their data and applies the events they
  return.
- Rules: the folder holds only views
  (`module_roots_and_documentation_describe_the_entire_source_tree` in
  `apps/ticketboard/src/tests/architecture_rules.rs`), and no other feature imports its `ui`
  (`dependency_boundaries_and_external_test_placement_are_enforced`).
