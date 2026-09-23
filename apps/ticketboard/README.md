# `ticketboard` — native ticket registry workspace

Ticketboard is the egui/eframe desktop interface to `.ai/tickets/`. It displays all parent and child tickets, recorded wave lanes, the program tree, measured run receipts, and historical estimates. Ticket details and repository Markdown documents remain available beside the board.

The application reads registry data through `ticket-engine`. Every ticket mutation runs an existing `cargo xtask ticket` command; direct writes are limited to eframe's user preferences outside the repository.

## Source atlas

```text
src/
├── main.rs              CLI arguments and native window startup.
├── application/         Lifecycle, composition, preferences, and action dispatch.
├── core/                Processes, clocks, logs, and shared UI primitives.
├── ticket_registry/     Repository discovery, corpus loading, and shared ticket models.
├── ticket_browser/      Board, tree, filters, details, and comparisons.
├── ticket_actions/      CLI commands, change guards, queue, dialogs, and feedback.
├── wave_plan/           Recorded lock, lane projections, and ownership collisions.
├── execution_metrics/   Separate measured receipts and estimated-token subsystems.
├── document_viewer/     Contained document reads, viewer state, and Markdown UI.
├── repository_status/   Strict checks, Git status, watching, and refresh scheduling.
└── tests/               Architecture checks and shared test fixtures.
```

Each module's README inventories its files and describes its responsibility, interface, and dependency rules:

- [application](src/application/README.md)
- [core](src/core/README.md)
- [ticket_registry](src/ticket_registry/README.md)
- [ticket_browser](src/ticket_browser/README.md)
- [ticket_actions](src/ticket_actions/README.md)
- [wave_plan](src/wave_plan/README.md)
- [execution_metrics](src/execution_metrics/README.md)
- [document_viewer](src/document_viewer/README.md)
- [repository_status](src/repository_status/README.md)

## Dependency and placement rules

- `application` composes features. Features never import application internals.
- `core` contains reusable primitives and imports no ticket domain or feature.
- `ticket_registry` exposes data to consumers and does not depend on them.
- Models and services remain independent of egui. Cross-feature reuse goes through models, services, or event contracts; features never import another feature's UI.
- Feature rendering receives narrow borrowed views and emits feature events. The application supplies the browser's ticket-menu and action-strip callbacks, preserving the UI's composition without coupling feature renderers.
- Put domain-specific work in its owning feature. Group related variants, detail sections, and dialogs into named subfolders; keep module entry files small.
- Production Rust files contain **fewer than 500 raw lines**, including comments and blanks. Test files contain **at most 1,000**. There are no ticket-board size exemptions.
- Comments describe current behavior and invariants. Unit tests live in sibling `tests/` files, declared with `#[cfg(test)] #[path = "tests/<name>.rs"] mod tests;`.

## Data and interaction flow

1. Startup resolves the repository from CLI arguments, current-directory discovery, or saved preferences. A native picker handles missing repositories.
2. Application loading gathers the corpus, wave lock, receipt metrics, vocabulary, and estimates on a worker thread. A malformed ticket refuses the whole corpus; wave and metric failures remain local to their displays.
3. Workspace construction precomputes cards, tree rows, facets, and aggregates. Reloads carry filters and independent sort choices and resolve selected tickets by ID.
4. A frame polls workers, paints feature views, then applies emitted actions. Document reads have their own state and discard stale results.
5. Ticket commands pass file-change guards and a single-flight queue. Completion refreshes data and Git status; failures retain output, drop pending commands, and request a strict check. Watch suppression prevents redundant checks during command writes.

Measured receipts and estimated tokens never share totals. Wave membership remains lock-verbatim. Document reads reject paths outside the repository, including symlink escapes, and use a bounded raw-text fallback when needed.

## Running

```bash
cargo run -p ticketboard -- [REPO_ROOT]
cargo run -p ticketboard -- --help
cargo run -p ticketboard --features glow -- [REPO_ROOT]
```

The default renderer is wgpu. The `glow` feature selects the OpenGL fallback. A desktop display and graphics driver are required for the native window. Preferences keep the selected repository and viewer width under the existing `repo_root` and `viewer_w` storage keys.

## Verification

```bash
cargo fmt -p ticketboard --check
cargo test -p ticketboard --locked
cargo test -p ticketboard --locked -- --ignored
cargo clippy -p ticketboard --locked --all-targets --all-features -- -D warnings
cargo build -p ticketboard --locked
cargo build -p ticketboard --locked --features glow
cargo xtask verify file-length
```

The normal suite uses temporary fixtures. The three ignored tests explicitly read the live checkout's corpus, estimates, and wave lock. Headless egui tests paint every tab, ticket details, mutation dialogs, refusal states, and document states without running ticket commands or requiring a native display.

[Architecture tests](src/tests/architecture_rules.rs) check file limits, module roots, complete README inventories, external test placement, and dependency direction. [Source inspection tests](src/tests/source_inspection.rs) support grouped imports and ignore comments and string literals. The repository-wide file-length gate also covers this crate.
