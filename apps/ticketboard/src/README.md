# Ticketboard source

The source tree of the [ticketboard](/documentation_v2/glossary.md#ticketboard) binary: the entry
point, one composition module, one module of shared foundations, and seven feature modules, each
owning one part of the viewer of the [ticket](/documentation_v2/glossary.md#ticket) registry.

## Contents

```text
apps/ticketboard/src/
├── application/        the eframe application: session state, preferences, background jobs, dispatch
├── core/               subprocesses, bounded logs, clock labels and the shared interface primitives
├── document_viewer/    repository documents read on a worker thread, shown as Markdown or raw text
├── execution_metrics/  measured run receipts and historical token estimates, kept apart
├── main.rs             the command line and the native window
├── repository_status/  the strict-check banner, the `git status` chip and the debounced file watch
├── tests/              the architecture tests, their source inspection and shared test fixtures
├── ticket_actions/     `cargo xtask ticket` commands, file-change guards, the queue, dialogs, feedback
├── ticket_browser/     the status board, program tree, filters, ticket details and comparisons
├── ticket_registry/    repository discovery, corpus loading and the ticket models every feature shares
└── wave_plan/          the recorded wave lock, its lanes as stored, and ownership collisions
```

## How it works

`main.rs` answers `--help` or `-h` with the usage, takes the first other argument as the
repository root, opens a 1500 by 950 window (720 by 480 at least) titled "Ticketboard" and hands
it `application::TicketboardApp::new`. The renderer is wgpu, or glow in a `--features glow`
build.

`application` composes everything else. Each feature keeps its data in `models/` and `services/`,
free of egui, and draws in `ui/` from a narrow borrowed view the application lends it; what the
viewer does there comes back as that feature's events (`events.rs`), which the application turns
into actions and applies after the frame. The application also hands the browser the ticket menus
and the detail panel's action strip of `ticket_actions` as callbacks, so neither feature imports
the other's `ui`. `ticket_registry` is the shared data every feature reads, and `core` holds what
any module may use.

```text
main.rs ──▶ application ──▶ the six features: ticket_browser, ticket_actions, wave_plan,
                │            execution_metrics, document_viewer, repository_status
                │                          │ through models, services and events
                ▼                          ▼
          ticket_registry ◀────────────────┘

every module but core and document_viewer ──▶ ticket_engine (tools_v2/ticket-engine)
any module ──▶ core (process, time, ui)
```

## Public surface

None: the crate is a binary, and no module is visible outside it. The `ticketboard` executable
and its argument are described in the crate README.

## Boundaries

- Depends on: `ticket_engine` for the ticket model, validation, repository paths and the wave
  lock; `eframe`, `egui_commonmark`, `egui_extras`, `notify`, `rfd`, `serde`, `serde_json`,
  `time` and `toml`; at run time `cargo xtask ticket` and `git`, run as subprocesses.
- Used by: nothing in the repository links it; people run the binary.
- Rules: each held by a test in `tests/architecture_rules.rs`:
  - the top level holds only `main.rs`, this README, `tests/` and the nine module folders, each
    with a `mod.rs`, and the crate README exists
    (`module_roots_and_documentation_describe_the_entire_source_tree`);
  - `core` imports no feature and nothing from `ticket_engine`; no feature imports `application`;
    a feature imports no other feature's `ui`, `core::ui` excepted; `ticket_registry` imports no
    consuming feature; `models/` and `services/` (and `execution_metrics`' `measured/` and
    `estimated/`) never name egui; no source declares an inline module or unit test
    (`dependency_boundaries_and_external_test_placement_are_enforced`);
  - a production file stays under 500 lines and a test file at or under 1000
    (`source_files_respect_the_size_limits_without_exemptions`), which
    `cargo xtask verify file-length` checks as well.
