# Operations pages

The `operations_pages` crate: the [operations](/documentation/glossary/n_to_z.md#operations)
pages of the single-page app, the second section of the sidebar — the schedule of upcoming
[events](/documentation/glossary/a_to_f.md#event), each event's hub with its inline
[ORBAT](/documentation/glossary/n_to_z.md#orbat) slotting, one
[mission](/documentation/glossary/g_to_m.md#mission)'s slotting on a page of its own, the viewer's
own [service record](/documentation/glossary/n_to_z.md#service-record) with the leave panels, and
the global leaderboards.

## Contents

```text
crates/frontend/pages/operations_pages/
├── Cargo.toml  the package: `frontend_session`, `frontend_transport`, `frontend_api_dtos`, `frontend_ui`, `http_url_guard`, `leptos`, `serde_json`, layout tier 10
└── src/        the five pages, the panels they are built from, and their guard tests
```

## How it works

The app's route table (`apps/frontend/src/app_routes.rs`) mounts the five route components:
`EventSchedulePage` at `/events`, `EventHubPage` at `/events/:id`, `OrbatSelectionPage` at
`/events/:id/missions/:emid/orbat`, `DeploymentsPage` at `/deployments` and `LeaderboardsPage` at
`/leaderboards`. Each renders inside the session's sign-in gate and fetches through the transport
crate's typed calls. The event hub has one renderer and the slotting one implementation: the
schedule's detail column renders the same hub body as the event hub route, and the ORBAT selection
page mounts the hub's slotting selector. The [source tree README](src/README.md) walks through each
page.

The route components fetch in the browser only, so they are compiled for `wasm32` alone; the pure
readers under them (the viewer's standing on a mission, the place outlook, the refusal notices, the
leaderboard row parsing, the service record's links) compile on every target, so their tests run
natively.

## Getting started

Run from the repository root:

```bash
cargo test -p operations_pages   # the registration access readers, the guard tests over the page source
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `schedule::EventSchedulePage`, `event_detail::EventHubPage`,
  `orbat_selection::OrbatSelectionPage`, `deployments::DeploymentsPage` and
  `leaderboards::LeaderboardsPage`: the route components (`wasm32`).
- The components the pages mount from another module are public items of public modules, as
  leptos's derived props builders need (`wasm32`): `deployments::{leave_of_absence,
  leave_review_queue, table_head}` (`LeaveOfAbsencePanel`, `AdminLeaveQueue`, `ServiceHead`),
  `event_detail::{assign_picker, slotting_selector}` (`AssignPicker`, `OrbatSelector`, with
  `event_detail::registration_access::mission_standing::MissionStanding`) and
  `leaderboards::operator_dossier` (`OperatorDossier`, with `leaderboards::page::Row`); each
  route component's `page` module is public too.
- `prelude`: the five route components.

## Boundaries

- Depends on: `frontend_session` (the `AuthStore` session, the role checks and the sign-in gate),
  `frontend_transport` (the API client and the event registration helpers), `frontend_api_dtos`
  (the wire types), `frontend_ui` (the interface primitives and the date, countdown and avatar
  helpers), `http_url_guard` (the scheme guard on rendered links and images), `leptos`,
  `serde_json`; on `wasm32`, `leptos_router` and `js-sys`; `frontend_test_support` for its tests
  only.
- Used by: the single-page app (`apps/frontend`), whose route table mounts the five pages.
- Rules: a page crate depends on foundation and feature crates only, never on another page crate,
  a workspace or the app (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Source tree](src/README.md) — the pages, their routes and the rules that keep them aligned.
- [Operations pages documentation](/documentation/crates/frontend/pages/operations_pages/README.md)
  — the feature docs of each page.
