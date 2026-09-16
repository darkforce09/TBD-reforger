# data

Mission data: the authored scenario and the CRDT store that edits it. Folded in from the deleted
`website-mission-core` crate at T-0xx Phase 2A.

## Contents

- `scenario`: authored payloads, the compiler, validation, authored extensions, and plain-text
  ORBAT slot lines. Gated on `scenario`, which is this crate's default feature and the only tier
  `website-api` enables.
- `store`: the `yrs` layer — ordered arrays and undo clocks (`crdt`), row projections (`rows`),
  headless authored-document commands (`operations`), and row-to-identity resolution
  (`selection.rs`). Gated on `store`.

## Boundaries

`scenario` is immutable input and pure computation: no browser APIs, no graphics devices, no Leptos
state. Callers provide domain data explicitly. Wire keys, numeric conversions, authored order,
diagnostics, and resource substitutions are preserved. Host interaction state stays in the
frontend; document operations take explicit values and callbacks.
