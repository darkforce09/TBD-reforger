# Administration pages

The [administration](/documentation/glossary/a_to_f.md#administration) pages, the last section of the
sidebar and the only one reserved for the `admin` [role](/documentation/glossary/n_to_z.md#role):
the operations calendar of [events](/documentation/glossary/a_to_f.md#event), the
[mission](/documentation/glossary/g_to_m.md#mission) approval queue, the game servers, the member
roster, the announcements, the audit trail and the ballistics catalogs.

## Contents

```text
apps/frontend/src/pages/administration/
├── approvals/           the mission approval queue: the artifact under review and the decision on it
├── audit_logs/          the administrative action trail, live and page by page, and one entry's record
├── ballistics_catalogs/  ballistics catalog versions: upload with calibration, validation report, list
├── content_manager/     announcements: write, publish, push to Discord and archive them
├── event_manager/       the operations calendar: schedule, edit and delete events, and set who may join
├── mod.rs               the module tree; re-exports the seven route components
├── personnel/           the paged member roster and one member's dossier: bans, warnings, role resync
└── server_control/      the game servers: fleet commands, deployments, fleet scenarios and credentials
```

## How it works

| Sidebar entry | Route | Component |
|---|---|---|
| "Event Manager" | `/admin/events` | `EventManagerPage` in `event_manager/` |
| "Mission Approvals" | `/admin/approvals` | `MissionApprovalsPage` in `approvals/` |
| "Server Control" | `/admin/server` | `ServerControlPage` in `server_control/` |
| "Personnel Roster" | `/admin/personnel` | `PersonnelRosterPage` in `personnel/` |
| "Comms Broadcaster" | `/admin/content` | `ContentManagerPage` in `content_manager/` |
| "Audit Logs" | `/admin/audit` | `AuditLogsPage` in `audit_logs/` |
| "Ballistics Catalogs" | `/admin/ballistics-catalogs` | `BallisticsCatalogsPage` in `ballistics_catalogs/` |

Every route declares the `admin` tier, and the sidebar hides the Administration section from any
other role. The route guard redirects no one from an `admin` route: each page wraps its body in
`AdminGate`, which shows "Loading session…" while the session restores, a sign-in prompt to a
signed-out viewer and "Admin access required." below `admin`, in place of the page. Each page owns
its own fetches and signals, and nothing is shared between pages. Every request runs in the browser
build only; the views that run them exist in that build only.

The pages reach five [API](/documentation/glossary/a_to_f.md#api) domains: `operations` for events,
their missions and their access, and for the ballistics catalogs; `missions` for
[approvals](/documentation/glossary/a_to_f.md#approvals), reviews, the library and mission
[deployments](/documentation/glossary/g_to_m.md#mission-deployment); `server_infrastructure` for
servers, [fleet commands](/documentation/glossary/a_to_f.md#fleet-command),
[fleet scenarios](/documentation/glossary/a_to_f.md#fleet-scenario) and
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential); `administration` for the
paged roster, bans, warnings, the role resync, the audit trail and its live stream;
`community_content` for announcements and uploads.

## Public surface

- `AuditLogsPage`, `BallisticsCatalogsPage`, `ContentManagerPage`, `EventManagerPage`,
  `MissionApprovalsPage`, `PersonnelRosterPage` and `ServerControlPage`: the route components
  `apps/frontend/src/app_routes.rs` mounts, each imported from its own module; `mod.rs`
  also re-exports all seven. Each child's README gives its route, tier and layout.

## Boundaries

- Depends on: `crate::foundation` (the API client, its DTOs and endpoint modules, the `AuthStore`
  session, `AdminGate` and the other UI primitives, the date helpers); the review views of
  `apps/frontend/src/features/mission_review_record/`, which the approval queue and
  the deployments panel reuse; over HTTP, the five API domains above.
- Used by: the route table in `apps/frontend/src/app_routes.rs`, whose tiers, layout flags
  and breadcrumbs `apps/frontend/src/foundation/route_table/mod.rs` declares; the Administration section of
  `apps/frontend/src/foundation/route_table/navigation_menu.rs`; the source pins in
  `apps/frontend/src/foundation/test_support/pins.rs`; the DOM oracle captures in
  `tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs`.
- Rules: every page renders its body inside `AdminGate`, and an `admin` route has no denial
  redirect (`denial_redirects_to_overview_with_role_notice` in
  `apps/frontend/src/foundation/route_table/tests/route_authorization.rs`); every route here declares the `admin`
  tier and every sidebar entry of the section asks for `Role::Admin`; a page's own rules and their
  tests are in its README.

## Related documentation

- [Administration pages](/documentation/apps/frontend/pages/administration/README.md) — the
  six pages' feature docs and design references.
- [Administration domain](/apps/api/src/administration/README.md) — the roster,
  moderation and audit routes.
