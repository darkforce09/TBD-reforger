# Administration pages

The `administration_pages` crate: the [administration](/documentation/glossary/a_to_f.md#administration)
pages of the single-page app, the last section of the sidebar and the only one reserved for the
`admin` [role](/documentation/glossary/n_to_z.md#role) — the operations calendar of
[events](/documentation/glossary/a_to_f.md#event) with each event's access panel, the
[mission](/documentation/glossary/g_to_m.md#mission) approval queue, server control with its
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command),
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment),
[fleet scenarios](/documentation/glossary/a_to_f.md#fleet-scenario) and
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential), the member roster,
the announcements, the audit trail and the ballistics catalogs.

## Contents

```text
crates/frontend/pages/administration_pages/
├── Cargo.toml  the package: `mission_review_record`, `frontend_session`, `frontend_transport`, `frontend_api_dtos`, `frontend_ui`, `frontend_route_table`, `leptos`, layout tier 11
└── src/        the seven pages, the panels they are built from, and their guard tests
```

## How it works

The app's route table (`crates/frontend/shell/frontend_application/src/app_routes.rs`) mounts the seven route components:
`EventManagerPage` at `/admin/events`, `MissionApprovalsPage` at `/admin/approvals`,
`ServerControlPage` at `/admin/server`, `PersonnelRosterPage` at `/admin/personnel`,
`ContentManagerPage` at `/admin/content`, `AuditLogsPage` at `/admin/audit` and
`BallisticsCatalogsPage` at `/admin/ballistics-catalogs`. Each wraps its body in the session's
`AdminGate`, owns its own fetches and signals, and shares nothing with the other pages. The
approval queue and the mission deployments panel render a review through the
`mission_review_record` crate, so an administrator reads the record in the author's words. The
[source tree README](src/README.md) walks through each page.

Every request runs in the browser build only, so the route components and the views that fetch
are compiled for `wasm32` alone. The wording, the view models, the refusal readings and the
request builders under them are compiled for `wasm32` and for the native tests, and nothing else
of the crate reads them natively.

## Getting started

Run from the repository root:

```bash
cargo test -p administration_pages   # the wording and view models, the guard tests over the page source
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `event_manager::EventManagerPage`, `approvals::MissionApprovalsPage`,
  `server_control::ServerControlPage`, `personnel::PersonnelRosterPage`,
  `content_manager::ContentManagerPage`, `audit_logs::AuditLogsPage` and
  `ballistics_catalogs::BallisticsCatalogsPage`: the route components (`wasm32`), each also
  reachable at its page module (`<area>::page::<Component>`), so the props types leptos derives
  for it are public.
- `approvals::review_drawer::ReviewInspector` with `approvals::page::ApprovalsDesk`, and
  `event_manager::access::member_search::MemberSearch`: the components sibling modules share
  (`wasm32`), public with their props types so no item leptos derives is unreachable.
- `prelude`: the seven route components.

## Boundaries

- Depends on: `mission_review_record` (the review history, thread, provenance and wording),
  `frontend_session` (the `AuthStore` session and `AdminGate`), `frontend_transport` (the API
  client, the typed endpoint calls, the multipart upload and the audit stream), `frontend_api_dtos`
  (the wire types and their typed identifiers), `frontend_ui` (the interface primitives and the
  date, clipboard and formatting helpers), `frontend_route_table` (the route paths the pages link
  to), `time_source` (the clock and UTC formatter of new posts), `leptos`, `futures`,
  `serde_json`, `url`; on `wasm32`, `leptos_router`, `gloo-timers`,
  `js-sys`, `wasm-bindgen` and `web-sys`; `frontend_test_support` for its tests only.
- Used by: the single-page app (`crates/frontend/shell/frontend_application`), whose route table mounts the seven pages.
- Rules: a page crate depends on foundation and feature crates only, never on another page crate,
  a workspace or the app (`cargo xtask ci verify-workspace-laws`); every page renders its body
  inside `AdminGate`, and every route of the crate declares the `admin` tier in
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`.

## Related documentation

- [Source tree](src/README.md) — the pages, their routes and the API domains they reach.
- [Administration pages documentation](/documentation/crates/frontend/pages/administration_pages/README.md)
  — the feature docs and design references of each page.
