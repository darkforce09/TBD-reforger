# Mission hub pages

The [mission](/documentation/glossary/g_to_m.md#mission) hub, the third section of the sidebar: the
library of missions, one mission's overview, and the New Mission dialog that hands an author to
the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator). The review record these
pages show lives in `apps/frontend/src/features/mission_review_record/`, and the read-only review
workspace of an [artifact](/documentation/glossary/a_to_f.md#artifact) in
`apps/frontend/src/workspaces/editor/review_workspace/`.

## Contents

```text
apps/frontend/src/pages/mission_hub/
├── create_dialog/  the New Mission dialog over the library, which hands the author to the editor
├── library/        the mission catalogue with its slide-over dossier
├── mod.rs          the module tree
└── overview/       one mission's dossier and its Edit Armory dialog
```

## How it works

| Page title | Route | Component |
|---|---|---|
| "Mission Library" | `/missions` | `MissionLibraryPage` in `library/` |
| "Mission Overview" | `/missions/:id` | `MissionOverviewPage` in `overview/` |

Every page renders its content inside `AuthGate`. The create dialog has no route of its own; it
opens over the library, and a created mission loads the Mission Creator at `/missions/{id}/edit`,
the route the library's dossier also opens. The overview's `dossier_body` is the one rendering of a
mission's facts, and the library's slide-over renders it too, so it stays free of authoring
controls: the "Edit Armory" dialog lives beside it on the overview route, and the review record sits
under it, never inside it.

## Public surface

- `MissionLibraryPage` and `MissionOverviewPage`: the route components
  `apps/frontend/src/app_routes.rs` mounts; each child's README gives its route, tier and
  layout.

## Boundaries

- Depends on: `crate::foundation` (the [API](/documentation/glossary/a_to_f.md#api) client, its DTOs,
  the `AuthStore` session and [role](/documentation/glossary/n_to_z.md#role) checks, the UI
  primitives, `format_bytes` for the upload size); `crate::features::mission_review_record`
  (`MissionReviewRecord`, `SubmitForReview`); `mission_payload` for the upload
  body; it imports no workspace and links to the review workspace only by its route; over HTTP, the API's `missions`
  domain (`/api/v1/missions` and its children).
- Used by: the route table in `apps/frontend/src/app_routes.rs`, whose tiers and layout
  flags `apps/frontend/src/foundation/route_table/mod.rs` declares; the source pins in
  `apps/frontend/src/foundation/test_support/pins.rs`.
- Rules: the shared dossier body stays read-only and the review record stays outside it
  (`the_card_badge_and_the_dossier_grid_share_one_label_mapper` in
  `overview/tests/mission_overview.rs` holds the one status label both pages use); every review
  renders through `crate::features::mission_review_record`, for the author and the reviewer alike.

## Related documentation

- [Mission library page](/documentation/apps/frontend/pages/mission_hub/library/mission_library_page.md)
  — the library's behaviour and design.
- [Mission overview page](/documentation/apps/frontend/pages/mission_hub/overview/mission_overview_page.md)
  — the overview's behaviour and design.
- [Review workspace page](/documentation/apps/frontend/workspaces/editor/review_workspace/review_workspace_page.md)
  — the read-only review workspace these pages link to.
- [Mission hub pages documentation](/documentation/apps/frontend/pages/mission_hub/README.md)
  — the index of the pages' feature docs.
- [Missions domain](/apps/api/src/missions/README.md) — the API routes these pages call.
