# `document_viewer/`

## Responsibility

Loads repository-contained documents on worker threads and displays Markdown or a clearly labeled raw-text fallback.

## Public surface

`models::ViewerState` accepts only the active request's result. `services::document_loading` classifies paths, enforces the repository boundary, caps reads, and reports results. UI emits `DocumentEvent` for closing or opening a document externally.

## Dependency rules

Models and readers contain no egui types; the application owns the render cache and saved column width. File reads are bounded, canonicalization prevents symlink escapes, and stale results never replace the active document. Closing this feature does not alter ticket selection.

## Files

- [events.rs](events.rs) — Events.
- [mod.rs](mod.rs) — Module interface and composition.
- [models/mod.rs](models/mod.rs) — Module interface and composition.
- [models/read_outcome.rs](models/read_outcome.rs) — Read outcome.
- [models/viewer_state.rs](models/viewer_state.rs) — Viewer state.
- [services/document_loading.rs](services/document_loading.rs) — Document loading.
- [services/mod.rs](services/mod.rs) — Module interface and composition.
- [services/tests/document_loading.rs](services/tests/document_loading.rs) — Tests for document loading.
- [ui/document_column.rs](ui/document_column.rs) — Document column.
- [ui/mod.rs](ui/mod.rs) — Module interface and composition.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
