# `repository_status/`

## Responsibility

Projects strict-check progress and Git status, and observes registry and generated-document changes for debounced refreshes.

## Public surface

`models::check_status` tracks build/check phases and coalesced reruns. `models::git_status` classifies porcelain output. `services::file_watch` manages watched paths and debounce/suppression rules. `StatusView` provides read-only banner data; `StatusEvent` requests user actions.

## Dependency rules

Models and watch services remain independent of rendering and application state. The application owns process orchestration and applies watch results. Required-watch failure and supplementary-watch degradation stay visible. Only an observed successful check exit reports success.

## Files

- [events.rs](events.rs) — Events.
- [mod.rs](mod.rs) — Module interface and composition.
- [models/check_status.rs](models/check_status.rs) — Check status.
- [models/git_status.rs](models/git_status.rs) — Git status.
- [models/mod.rs](models/mod.rs) — Module interface and composition.
- [models/tests/check_status.rs](models/tests/check_status.rs) — Tests for check status.
- [models/tests/git_status.rs](models/tests/git_status.rs) — Tests for git status.
- [models/view.rs](models/view.rs) — View.
- [services/file_watch.rs](services/file_watch.rs) — File watch.
- [services/mod.rs](services/mod.rs) — Module interface and composition.
- [services/tests/file_watch.rs](services/tests/file_watch.rs) — Tests for file watch.
- [ui/mod.rs](ui/mod.rs) — Module interface and composition.
- [ui/status_banner.rs](ui/status_banner.rs) — Status banner.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
