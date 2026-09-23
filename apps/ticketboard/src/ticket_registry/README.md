# `ticket_registry/`

## Responsibility

Discovers repository roots, loads every parent and child ticket, and exposes shared ticket fields, classifications, scope breadcrumbs, and semantic colors.

## Public surface

`services::discovery` resolves CLI and current-directory repository choices. `services::corpus_loading::load_corpus` returns a complete corpus or a named parse refusal. `models` exposes corpus types and reusable ticket projections.

## Dependency rules

Depends on ticket-engine and standard parsing/filesystem facilities, never on its consuming features. Models contain no UI types. Registry loading does not coordinate wave, metric, or document loading and never writes ticket files.

## Files

- [mod.rs](mod.rs) — Module interface and composition.
- [models/classification.rs](models/classification.rs) — Classification.
- [models/corpus.rs](models/corpus.rs) — Corpus.
- [models/mod.rs](models/mod.rs) — Module interface and composition.
- [models/palette.rs](models/palette.rs) — Palette.
- [models/projection.rs](models/projection.rs) — Projection.
- [models/scope.rs](models/scope.rs) — Scope.
- [services/corpus_loading.rs](services/corpus_loading.rs) — Corpus loading.
- [services/discovery.rs](services/discovery.rs) — Discovery.
- [services/mod.rs](services/mod.rs) — Module interface and composition.
- [services/tests/corpus_loading.rs](services/tests/corpus_loading.rs) — Tests for corpus loading.
- [services/tests/discovery.rs](services/tests/discovery.rs) — Tests for discovery.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
