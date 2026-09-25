# Mission hub pages

The [mission](/documentation_v2/glossary.md#mission) hub, the third section of the sidebar: the
library of missions, one mission's overview, the read-only review workspace of an
[artifact](/documentation_v2/glossary.md#artifact), the New Mission dialog that hands an author to
the [Mission Creator](/documentation_v2/glossary.md#mission-creator), and the review record these
pages share with the [approvals](/documentation_v2/glossary.md#approvals) page.

## Contents

```text
apps/website/frontend/src/v2/pages/mission_hub/
├── create_dialog/     the New Mission dialog over the library, which hands the author to the editor
├── library/           the mission catalogue with its slide-over dossier
├── mission_review/    the review record, submit control and wording shared with the approvals page
├── mod.rs             the module tree
├── overview/          one mission's dossier and its Edit Armory dialog
└── review_workspace/  the Mission Creator, read-only, on the version an artifact compiled from
```

## How it works

| Page title | Route | Component |
|---|---|---|
| "Mission Library" | `/missions` | `MissionLibraryPage` in `library/` |
| "Mission Overview" | `/missions/:id` | `MissionOverviewPage` in `overview/` |
| "Review Workspace" | `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` in `review_workspace/` |

Every page renders its content inside `AuthGate`. The create dialog has no route of its own; it
opens over the library, and a created mission loads the Mission Creator at `/missions/{id}/edit`,
the route the library's dossier also opens. The overview's `dossier_body` is the one rendering of a
mission's facts, and the library's slide-over renders it too, so it stays free of authoring
controls: the "Edit Armory" dialog lives beside it on the overview route, and the review record sits
under it, never inside it. `mission_review/` is the one rendering of a review, a thread entry and a
compile finding, for the author here and the reviewer on the approvals page. The review workspace is
the Mission Creator itself in its read-only review mode, not a copy of it.

## Public surface

- `MissionLibraryPage`, `MissionOverviewPage` and `ReviewWorkspacePage`: the route components
  `apps/website/frontend/src/app_routes.rs` mounts; each child's README gives its route, tier and
  layout.
- `review_wording`, `artifact_provenance_view`, `comment_composer` and `review_history_view` in
  `mission_review/`: the review wording and views the approvals page in
  `apps/website/frontend/src/v2/pages/administration/approvals/` renders; the [server
  control](/documentation_v2/glossary.md#server-control) page's [mission
  deployment](/documentation_v2/glossary.md#mission-deployment) wording reads
  `review_wording::short_digest`.

## Boundaries

- Depends on: `crate::v2::core` (the [API](/documentation_v2/glossary.md#api) client, its DTOs and
  the `mission_reviews` endpoint helpers, the `AuthStore` session and
  [role](/documentation_v2/glossary.md#role) checks, the UI primitives); the Mission Creator in
  `apps/website/frontend/src/v2/apps/editor/`, which `review_workspace/page.rs`,
  `library/dossier_upload.rs` and `library/dossier_upload_panel.rs` import directly;
  `website_map_engine::data::scenario::compile` for the upload body; over HTTP, the API's `missions`
  domain (`/api/v1/missions` and its children).
- Used by: the route table in `apps/website/frontend/src/app_routes.rs`, whose tiers and layout
  flags `apps/website/frontend/src/router.rs` declares; the approvals and server control pages in
  `apps/website/frontend/src/v2/pages/administration/`; the source pins in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: the shared dossier body stays read-only and the review record stays outside it
  (`the_card_badge_and_the_dossier_grid_share_one_label_mapper` in
  `overview/tests/mission_overview.rs` holds the one status label both pages use); every review
  renders through `mission_review/`, for the author and the reviewer alike.

## Related documentation

- [Mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md)
  — the library's behaviour and design.
- [Mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md)
  — the overview's behaviour and design.
- [Review workspace page](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md)
  — the review workspace's behaviour.
- [Mission hub pages documentation](/documentation_v2/website/frontend/pages/mission_hub/README.md)
  — the index of the three pages' feature docs.
- [Missions domain](/apps/website/api_v2/src/missions/README.md) — the API routes these pages call.
