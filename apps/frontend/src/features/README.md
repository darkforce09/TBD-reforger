# Shared features

The features layer of the single-page app: product capabilities that more than one page or
workspace shows, each rendering one concept (its views, controls and wording) the same way
wherever it appears.

## Contents

```text
apps/frontend/src/features/
├── mission_review_record/  a mission's review history, thread, provenance and submit control
└── mod.rs                  the module tree
```

## How it works

A feature fetches through `apps/frontend/src/foundation/` and renders components that pages and
workspaces mount. The [mission](/documentation/glossary/g_to_m.md#mission) review record is shown
by the mission hub's overview and library dossier, by the approvals queue and the server control
page's deployment wording, and by the Mission Creator's read-only review workspace, so the author
and the reviewer read one record in the same words.

## Public surface

- `mission_review_record`: `MissionReviewRecord`, `SubmitForReview`, the comment composer, the
  history and provenance views and the pure review wording; see its README.

## Boundaries

- Depends on: `apps/frontend/src/foundation/` (the transport, the interface primitives, the
  utilities) and `map_engine` through the wire types.
- Used by: the pages under `apps/frontend/src/pages/` and the workspaces under
  `apps/frontend/src/workspaces/`.
- Rules: a feature never imports a page, a workspace or the shell (`cargo xtask verify
  frontend-layering`, layer order foundation < features < pages, workspaces < shell).

## Related documentation

- [Frontend source root](/apps/frontend/src/README.md) — the five layers and their import order.
