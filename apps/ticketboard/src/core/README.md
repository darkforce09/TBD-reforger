# `core/`

## Responsibility

Reusable process execution, bounded logs, time formatting, external path opening, and small UI primitives.

## Public surface

`process::spawn_streaming` streams merged output and process completion through a channel. `ProcessHandle` supports cancellation; `BoundedLog` reports dropped lines. `time` supplies wall-clock labels. `ui` supplies shared colors and identifier links.

## Dependency rules

Core imports no application feature or ticket-engine domain types. UI primitives may depend on egui; process and time helpers remain independent of rendering. Background workers receive wake callbacks rather than application state.

## Files

- [mod.rs](mod.rs) — Module interface and composition.
- [process/bounded_log.rs](process/bounded_log.rs) — Bounded log.
- [process/cargo_discovery.rs](process/cargo_discovery.rs) — Cargo discovery.
- [process/external_open.rs](process/external_open.rs) — External open.
- [process/mod.rs](process/mod.rs) — Module interface and composition.
- [process/streaming.rs](process/streaming.rs) — Streaming.
- [process/tests/process.rs](process/tests/process.rs) — Tests for process.
- [time.rs](time.rs) — Time.
- [ui/mod.rs](ui/mod.rs) — Module interface and composition.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
