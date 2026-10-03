# ticketboard_model source

The modules of the ticketboard's headless half: one per viewer feature, the ticket-free `core`,
and the egui-free application state.

## Contents

```text
tools/tickets/ticketboard_model/src/
├── application_state/  the tabs and actions, the loaded board, reloads, column layout, background load
├── core/               streamed subprocesses, bounded logs, cargo discovery, external open, clock labels
├── document_viewer/    repository documents read on a worker thread, fenced to the repository root
├── error.rs            `Error` and `Result`: the document and wave-lock refusals
├── execution_metrics/  measured run receipts and historical token estimates, kept apart
├── lib.rs              the crate root: module tree and the error re-export
├── prelude.rs          the names `use ticketboard_model::prelude::*;` brings in
├── repository_status/  the strict-check and `git status` models and the debounced file watch
├── tests/              the architecture rules, the registry round trip and the shared test fixtures
├── ticket_actions/     `cargo xtask ticket` commands, guards, the queue, dialogs and toasts
├── ticket_browser/     the board, the program tree, detail sections, filters and scope facets
├── ticket_registry/    repository discovery, corpus loading and the ticket models every feature reads
└── wave_plan/          the recorded wave lock, its lanes as stored, and ownership collisions
```

## Boundaries

- Depends on: the crates named in the crate README.
- Used by: `apps/ticketboard`.
