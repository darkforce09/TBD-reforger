# Command center pages

The `command_center_pages` crate: the [command center](/documentation/glossary/a_to_f.md#command-center)
pages of the single-page app, the first section of the sidebar — the read-only screens a member
lands on after signing in: the landing dashboard (the next
[event](/documentation/glossary/a_to_f.md#event), the game server, the viewer's assignment, the
modpack and the latest announcements), one game server's live panel, and the announcement board.

## Contents

```text
crates/frontend/pages/command_center_pages/
├── Cargo.toml  the package: `frontend_session`, `frontend_transport`, `frontend_api_dtos`, `frontend_ui`, `http_url_guard`, `leptos`, `serde_json`, layout tier 10
└── src/        the three pages, the panels they are built from, and their guard tests
```

## How it works

The app's route table (`crates/frontend/shell/frontend_application/src/app_routes.rs`) mounts the three route components:
`DashboardPage` at `/`, `ServerIntelPage` at `/server-intel` and `AnnouncementsPage` at
`/announcements` and `/announcements/:id`. Each renders inside the session's sign-in gate, owns one
fetch and renders that one payload; none of them writes platform data. The server intel page alone
holds a live connection, its server's [SSE](/documentation/glossary/n_to_z.md#sse) status stream,
which it aborts when the route unmounts. The [source tree README](src/README.md) walks through each
page.

The route components fetch in the browser only, so they are compiled for `wasm32` alone; the pure
helpers the board's tests pin (the paragraph split, the preview line, the thumbnail guard) compile
for `wasm32` and for the native tests.

## Getting started

Run from the repository root:

```bash
cargo test -p command_center_pages   # the board's text contract, the server golden, the guard tests over the page source
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `dashboard::DashboardPage`, `server_intel::ServerIntelPage` and
  `announcements::AnnouncementsPage`: the route components (`wasm32`), each also reachable at its
  page module (`<page>::page::<Component>`), so the props types leptos derives for it are public.
- `prelude`: the three route components.

## Boundaries

- Depends on: `frontend_session` (the `AuthStore` session and the sign-in gate),
  `frontend_transport` (the API client and the server status stream), `frontend_api_dtos` (the
  wire types), `frontend_ui` (the interface primitives and the date, clipboard and byte formatting
  helpers), `http_url_guard` (the scheme guard on announcement thumbnails), `leptos`,
  `serde_json`; on `wasm32`, `leptos_router`; `frontend_test_support` for its tests only.
- Used by: the single-page app (`crates/frontend/shell/frontend_application`), whose route table mounts the three pages.
- Rules: a page crate depends on foundation and feature crates only, never on another page crate,
  a workspace or the app (`cargo xtask ci verify-workspace-laws`); the pages only read (each calls
  `api_get` and nothing that writes), and every route of the crate declares the `none` tier in
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`.

## Related documentation

- [Source tree](src/README.md) — the pages, their routes and the API domains they read.
- [Command center pages documentation](/documentation/crates/frontend/pages/command_center_pages/README.md)
  — the feature docs and design references of each page.
