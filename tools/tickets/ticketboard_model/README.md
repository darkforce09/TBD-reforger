# ticketboard_model

The headless half of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard): everything
the desktop viewer of the [ticket](/documentation/glossary/n_to_z.md#ticket) registry knows, without
painting it. `apps/ticketboard` paints these models with egui.

## Contents

```text
tools/tickets/ticketboard_model/
├── Cargo.toml  the manifest: tier 4 of tools/tickets, no egui or eframe
└── src/        the feature models and services, the process helpers and the application state
```

## How it works

A worker thread loads the corpus, the wave lock, the run receipts, the estimates and the scope
vocabulary together (`application_state::background_loading`). `WorkspaceState` builds the board,
tree, filter, facet, wave-lane and metrics projections once per load and the filter verdicts once
per filter change, so a painted frame only reads. Each feature's views emit that feature's events,
which convert into `application_state::events::Action`. Ticket changes run as
`cargo xtask ticket <verb>` subprocesses behind a file-change guard and a single-flight queue; the
crate writes no file under the repository.

## Boundaries

- Depends on: `ticket_model` (the typed ticket, `TicketId`, `ScopeVocab`), `ticket_metrics` (the
  run receipt shape), `ticket_wave_lock` (the lock path, the missing-lock text, the collision
  rule), `repository_layout`, `time_source`; `notify`, `serde`, `serde_json`, `thiserror`, `time`,
  `toml`.
- Used by: `apps/ticketboard`, which also enables the `test_fixtures` feature from its
  dev-dependencies for the shared test helpers.
- Rules: no source names egui, eframe or rfd, `core` imports no feature, no feature imports
  `application_state`, and `ticket_registry` imports no consuming feature (the tests in
  `src/tests/architecture_rules.rs`); the registry round trip in `src/tests/registry_round_trip.rs`
  proves the board's readers render the committed tickets and wave lock back byte for byte.
