# Document column

The egui column in which the [ticketboard](/documentation_v2/glossary.md#ticketboard) shows one
repository document beside the ticket details.

## Contents

```text
apps/ticketboard/src/document_viewer/ui/
├── document_column.rs  `viewer_pane_ui`: the header, then progress, Markdown or the raw-text fallback
└── mod.rs              the module tree; re-exports `viewer_pane_ui`
```

## How it works

`viewer_pane_ui(ui, state, cache, repo_root, actions)` draws a header with "← Back", the
repository-relative path and, while a document is open, an "open externally" button, which
resolves the path against the repository root. Below it the body follows the `ViewerState`: a
spinner with "reading <path>…" while loading; the Markdown through `egui_commonmark` with the
cache the application owns; or the fallback note in the warning colour above the raw text, drawn
as virtualized monospace rows of `OUTPUT_ROW_H`, and "nothing was read" when the fallback holds no
text. Clicks come back as `DocumentEvent::CloseViewer` and `DocumentEvent::OpenPath`.

## Boundaries

- Depends on: `crate::document_viewer::events` and the `ViewerState` re-exported by
  `crate::document_viewer::services::document_loading`; `crate::core::ui` (`OUTPUT_ROW_H`);
  `eframe::egui` and `egui_commonmark`.
- Used by: `viewer_pane_ui` in `apps/ticketboard/src/application/feature_views.rs`, which lends
  the state, the render cache and the repository root and turns the events into actions.
- Rules: Back closes only the viewer column and leaves the ticket selection alone; no other
  feature imports this module (`dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`); the loading, rendered and fallback states
  paint headlessly in `apps/ticketboard/src/application/tests/rendering.rs`.
