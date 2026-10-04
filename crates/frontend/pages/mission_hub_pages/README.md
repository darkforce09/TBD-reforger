# Mission hub pages

The `mission_hub_pages` crate: the pages of the sidebar's Mission Hub section — the library of
[missions](/documentation/glossary/g_to_m.md#mission) with its slide-over dossier, one mission's
overview with its Edit Armory dialog, and the New Mission dialog that hands an author to the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator).

## Contents

```text
crates/frontend/pages/mission_hub_pages/
├── Cargo.toml  the package: the foundation crates, `mission_review_record`, `mission_payload`, `http_url_guard`, `leptos`, layout tier 11
└── src/        the library, the overview and the create dialog, with their tests and source pins
```

## How it works

The app's route table mounts `MissionLibraryPage` at `/missions` and `MissionOverviewPage` at
`/missions/:id`. The library fetches the catalogue, opens a mission's dossier in a slide-over sheet
and the New Mission dialog over itself, one overlay at a time; the overview fetches one mission
and renders the same read-only dossier body with the review record under it. The
[source tree README](src/README.md) walks through each folder.

The route components and every panel that fetches exist on `wasm32` only, because the endpoints
they call exist only in the browser build; the pure readers under them compile natively for the
tests too.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_hub_pages   # the payload comparison, the upload checks, the wording, the source guards
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `library::MissionLibraryPage` and `overview::MissionOverviewPage` (wasm32), also in `prelude`.
- `create_dialog::CreateMissionDialog` and `library::dossier_sheet::MissionDossierSheet`
  (wasm32): public only because the props builder leptos derives has `pub` methods.

## Boundaries

- Depends on: `frontend_api_dtos`, `frontend_transport`, `frontend_session`, `frontend_ui`,
  `mission_review_record`, `mission_payload`, `http_url_guard`, `leptos`, `leptos_router`,
  `serde_json`; the browser crates on `wasm32`; `frontend_test_support` for its tests only.
- Used by: the single-page app (`apps/frontend`), whose route table mounts the two pages.
- Rules: a page crate never depends on another page crate, a workspace or the app
  (`cargo xtask ci verify-workspace-laws`); the dossier body stays read-only, and every review
  renders through `mission_review_record`.

## Related documentation

- [Source tree](src/README.md) — each folder, its route and the rules its tests hold.
- [Mission hub pages documentation](/documentation/crates/frontend/pages/mission_hub_pages/README.md)
  — the pages' feature docs.
