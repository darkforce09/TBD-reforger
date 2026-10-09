# Frontend feature crates

The features layer of the frontend crates: product capabilities that more than one page or
workspace shows, each rendering one concept (its views, controls and wording) the same way
wherever it appears.

## Contents

```text
crates/frontend/features/
└── mission_review_record/  `mission_review_record`: a mission's review history, thread, provenance and submit control
```

## How it works

A feature crate fetches through the foundation crates and renders components that the app's pages
and workspaces mount. The [mission](/documentation/glossary/g_to_m.md#mission) review record is
shown by the mission hub's overview and library dossier, by the approvals queue and the server
control page's deployment wording, and by the Mission Creator's read-only review workspace, so the
author and the reviewer read one record in the same words.

## Public surface

- `mission_review_record`: `MissionReviewRecord`, `SubmitForReview`, the comment composer, the
  history and provenance views and the pure review wording; see its README.

## Boundaries

- Depends on: the foundation crates under `crates/frontend/foundation/` (the session, the
  transport, the wire types, the interface primitives).
- Used by: the page crates under `crates/frontend/pages/` and the workspace crates under
  `crates/frontend/workspaces/`.
- Rules: a feature crate never depends on a page, a workspace or the app (layer order foundation
  < features < pages, workspaces < shell, `cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Frontend crates](/crates/frontend/README.md) — the layer order every frontend crate follows.
- [Frontend source root](/crates/frontend/shell/frontend_application/src/README.md) — the app's entry point, route table and
  frame, and the layer order of the frontend crates.
