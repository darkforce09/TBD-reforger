# Doctrine pages

The `doctrine_pages` crate: the doctrine and info pages of the single-page app, the reference
section of the sidebar — the doctrine wiki of standard operating procedures and manuals with its
revision history, the vehicle database with each vehicle's dossier, and the
[modpack](/documentation/glossary/g_to_m.md#modpack) manifests a server expects a client to have.
Their data comes from the [community content](/documentation/glossary/a_to_f.md#community-content)
domain of the [API](/documentation/glossary/a_to_f.md#api).

## Contents

```text
crates/frontend/pages/doctrine_pages/
├── Cargo.toml  the package: `frontend_session`, `frontend_transport`, `frontend_api_dtos`, `frontend_ui`, `leptos`, `serde_json`, layout tier 10
└── src/        the three pages, the panels they are built from, and their guard tests
```

## How it works

The app's route table (`crates/frontend/shell/frontend_application/src/app_routes.rs`) mounts the three route components:
`WikiPage` at `/wiki` and `/wiki/:slug`, `VehicleDatabasePage` at `/vehicles` and `ModpacksPage`
at `/modpacks`. Each renders inside the session's sign-in gate, fetches its whole list once
through the transport crate and lays it out in the `GlassSplit` master-detail view, both panes
reading the one fetched list. Each page offers an administrator its writes: the wiki edits a
manual and restores an older revision, the vehicle database adds, edits and deletes vehicles, and
the modpacks page edits the packs. The [source tree README](src/README.md) walks through each
page.

The route components fetch in the browser only, so they are compiled for `wasm32` alone; the pure
readers under them (the wiki's block mapping, request paths and save refusals, the vehicle form's
validation, write requests, refusal wording and list updates) compile for `wasm32` and for the
native test build, so their tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p doctrine_pages   # the wiki's block mapping and save refusals, the vehicle form and writes, the guard tests over the page source
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `wiki::WikiPage`, `vehicles::VehicleDatabasePage` and `modpacks::ModpacksPage`: the route
  components (`wasm32`), each also reachable at its page module (`<page>::page::<Component>`), so
  the props type leptos derives for it is public.
- `prelude`: the three route components.

## Boundaries

- Depends on: `frontend_session` (the `AuthStore` session, the role checks and the sign-in gate),
  `frontend_transport` (the API client and its typed reads and writes), `frontend_api_dtos` (the
  wiki, vehicle and modpack wire types and the data envelope), `frontend_ui` (the split view,
  dialog and toast primitives, the safe URL and formatting helpers), `leptos`, `serde_json`; on
  `wasm32`, `leptos_router` and `serde`; `frontend_test_support` for its tests only.
- Used by: the single-page app (`crates/frontend/shell/frontend_application`), whose route table mounts the three pages.
- Rules: a page crate depends on foundation and feature crates only, never on another page crate,
  a workspace or the app (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Source tree](src/README.md) — the pages, their shared shape and the rules that keep them
  aligned.
- [Doctrine pages documentation](/documentation/crates/frontend/pages/doctrine_pages/README.md)
  — the feature docs and design references of each page.
