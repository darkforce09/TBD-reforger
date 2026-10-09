# Mission hub pages source

The source tree of `mission_hub_pages`: the [mission](/documentation/glossary/g_to_m.md#mission)
hub, the third section of the sidebar — the library of missions, one mission's overview, and the
New Mission dialog that hands an author to the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator). The review record these
pages show lives in `crates/frontend/features/mission_review_record/src/`, and the read-only review
workspace of an [artifact](/documentation/glossary/a_to_f.md#artifact) in
`crates/frontend/workspaces/mission_creator_workspace/src/review_workspace/`. The [crate README](../README.md) gives the
package, its commands and its boundaries.

## Contents

```text
crates/frontend/pages/mission_hub_pages/src/
├── create_dialog/  the New Mission dialog over the library, which hands the author to the editor
├── lib.rs          the crate root: the module tree
├── library/        the mission catalogue with its slide-over dossier
├── overview/       one mission's dossier and its Edit Armory dialog
└── prelude.rs      the two route components the app's route table mounts
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

The route components and every panel that fetches are compiled for `wasm32` only, since the
endpoints they call exist only in the browser build. The pure readers under them — the payload
comparison, the upload checks, the card and dossier wording, the armory draft guards — are
compiled for `wasm32` and for the native test build, where their tests run.

## Public surface

- `library::MissionLibraryPage` and `overview::MissionOverviewPage`: the route components
  `crates/frontend/shell/frontend_application/src/app_routes.rs` mounts, also in `prelude`; each child's README gives its
  route, tier and layout.
- `create_dialog::CreateMissionDialog` and `library::dossier_sheet::MissionDossierSheet`: public
  only because the props builder `#[component]` derives has `pub` methods; the library is their
  one caller.

## Boundaries

- Depends on: the foundation crates (the [API](/documentation/glossary/a_to_f.md#api) client, its DTOs,
  the `AuthStore` session and [role](/documentation/glossary/n_to_z.md#role) checks, the UI
  primitives, `format_bytes` for the upload size); the `mission_review_record` crate
  (`MissionReviewRecord`, `SubmitForReview`); `mission_payload` for the upload body; it imports
  no workspace and links to the review workspace only by its route; over HTTP, the API's
  `missions` domain (`/api/v1/missions` and its children).
- Used by: the route table in `crates/frontend/shell/frontend_application/src/app_routes.rs`, whose tiers and layout
  flags `crates/frontend/foundation/frontend_route_table/src/routes.rs` declares.
- Rules: the shared dossier body stays read-only and the review record stays outside it, and the
  card badge and the dossier grid share one status label; every review
  renders through the `mission_review_record` crate, for the author and the reviewer alike.

## Related documentation

- [Mission library page](/documentation/crates/frontend/pages/mission_hub_pages/library/mission_library_page.md)
  — the library's behaviour and design.
- [Mission overview page](/documentation/crates/frontend/pages/mission_hub_pages/overview/mission_overview_page.md)
  — the overview's behaviour and design.
- [Review workspace page](/documentation/crates/frontend/workspaces/mission_creator_workspace/review_workspace/review_workspace_page.md)
  — the read-only review workspace these pages link to.
- [Mission hub pages documentation](/documentation/crates/frontend/pages/mission_hub_pages/README.md)
  — the index of the pages' feature docs.
- [Missions domain](/crates/api/api_missions/src/README.md) — the API routes these pages call.
