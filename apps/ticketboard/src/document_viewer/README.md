# Document viewer views

The egui views of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s document viewer
feature: the document column: Markdown or raw text with its note, Back and open externally. The
feature's models, services and events are in
[`tools/tickets/ticketboard_model/src/document_viewer/`](/tools/tickets/ticketboard_model/src/document_viewer/README.md);
these views paint the borrowed view the application lends them and return the feature's events.

## Contents

```text
apps/ticketboard/src/document_viewer/
├── mod.rs     the module tree
└── ui/        the views
```

## Boundaries

- Depends on: `ticketboard_model::document_viewer` (its models and events) and the shared
  `ticketboard_model` models the views read; `crate::core::ui`; `eframe::egui`.
- Used by: `crate::application`, which lends the views their data and applies the events they
  return.
- Rules: the folder holds only views
  (`module_roots_and_documentation_describe_the_entire_source_tree` in
  `apps/ticketboard/src/tests/architecture_rules.rs`), and no other feature imports its `ui`
  (`dependency_boundaries_and_external_test_placement_are_enforced`).
