# Document viewer state

The state machine of the [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)'s document
column: which repository document is open, whether its read is still running, and what the read
produced.

## Contents

```text
apps/ticketboard/src/document_viewer/models/
├── mod.rs              the module tree; re-exports both modules' items
├── read_outcome.rs     `DocumentOutcome`, rendered or fallback, and `LoadedDocument`, tagged with its path
└── viewer_state.rs     `ViewerState` (closed, loading, rendered, fallback) with `open`, `close`, `land`
```

## Boundaries

- Depends on: `std` only.
- Used by: `crate::document_viewer::services::document_loading`, which re-exports the three types
  and builds `LoadedDocument`; through that re-export, `crate::application`, which holds the
  `ViewerState`, opens, closes and lands reads, and paints it with `crate::document_viewer::ui`.
- Rules: every state but `Closed` carries the repository-relative path as clicked, which is both
  the header label and the identity of a read; `land` applies a result only while `Loading` that
  same path, so a read superseded by another click or by Back is dropped
  (`stale_results_are_dropped` and `state_machine_transitions` in
  `apps/ticketboard/src/document_viewer/services/tests/document_loading.rs`); no egui type appears
  here (`dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`).
