**Status:** live

# Administration pages

The documentation of the seven [administration](/documentation/glossary/a_to_f.md#administration) pages
under `/admin/*`, one folder per page: each holds the page's feature doc and, where a design set
exists, its design references. Developers and AI agents read it before changing an administration
page.

## Contents

```text
documentation/crates/frontend/pages/administration_pages/
├── approvals/            the mission approvals page: reviewing the artifacts missions submit
├── audit_logs/           the audit logs page: the read-only trail of administrative actions
├── ballistics_catalogs/  the ballistics catalogs page: calibrated catalog uploads and stored versions
├── content_manager/      the content manager page, labelled Comms Broadcaster: announcements
├── event_manager/        the event manager page: the operations calendar and event access
├── personnel/            the personnel roster page: members, bans, warnings and the role resync
└── server_control/       the server control page: commands, deployments, scenarios and credentials
```

## How it works

The folders mirror the page folders under `crates/frontend/pages/administration_pages/src/` and
keep their spelling. Each holds a README index, the page's feature doc and, except for server
control and ballistics catalogs, a `visual_references/` folder with one or two design-phase blueprint sets. A feature doc
follows the [feature doc template](/documentation/standards/templates/feature_doc.md): Where it
lives, Behaviour (ending in the known discrepancies between the page and the
[API](/documentation/glossary/a_to_f.md#api), where there are any), Data (what each call means
server-side), Design (the layout as built and each difference from the blueprint), Open work and
Decisions. Start with the feature doc of the page at hand; its Design section leads to the
blueprint.

Every page is declared in `crates/frontend/foundation/frontend_route_table/src/routes.rs` for the `admin` tier, sits in the
sidebar's Administration section and renders its body inside `AdminGate`
(`crates/frontend/foundation/frontend_session/src/gates.rs`), so a viewer below the `admin`
[role](/documentation/glossary/n_to_z.md#role) sees "Admin access required." instead of the page.

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Event manager | `/admin/events`, `EventManagerPage` | Event Manager; heading "Operations Calendar" | [event_manager_page.md](/documentation/crates/frontend/pages/administration_pages/event_manager/event_manager_page.md) |
| Mission approvals | `/admin/approvals`, `MissionApprovalsPage` | Mission Approvals | [mission_approvals_page.md](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md) |
| Server control | `/admin/server`, `ServerControlPage` | Server Control | [server_control_page.md](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md) |
| Personnel roster | `/admin/personnel`, `PersonnelRosterPage` | Personnel Roster | [personnel_roster_page.md](/documentation/crates/frontend/pages/administration_pages/personnel/personnel_roster_page.md) |
| Content manager | `/admin/content`, `ContentManagerPage` | Comms Broadcaster | [content_manager_page.md](/documentation/crates/frontend/pages/administration_pages/content_manager/content_manager_page.md) |
| Audit logs | `/admin/audit`, `AuditLogsPage` | Audit Logs | [audit_logs_page.md](/documentation/crates/frontend/pages/administration_pages/audit_logs/audit_logs_page.md) |
| Ballistics catalogs | `/admin/ballistics-catalogs`, `BallisticsCatalogsPage` | Ballistics Catalogs | [ballistics_catalogs_page.md](/documentation/crates/frontend/pages/administration_pages/ballistics_catalogs/ballistics_catalogs_page.md) |

A new administration page gets a folder here named like its code folder, with a README, its
feature doc and, when a design set exists, a `visual_references/` folder; it also gets a line in
Contents and a row in the table.

## Code

- [Administration pages](/crates/frontend/pages/administration_pages/src/) — the seven route
  components and their panels, which the feature docs describe.
- [Administration domain](/crates/api/api_administration/src/) — the roster, bans, warnings,
  role resync and audit trail behind the personnel and audit logs pages.
- [Missions domain](/crates/api/api_missions/src/) — the approvals queue and decisions, and
  the [mission deployments](/documentation/glossary/g_to_m.md#mission-deployment) of server control.
- [Community content domain](/crates/api/api_community_content/src/) — the announcements and
  uploads the content manager writes.
- [Operations domain](/crates/api/api_operations/src/) — the
  [events](/documentation/glossary/a_to_f.md#event), their attached
  [missions](/documentation/glossary/g_to_m.md#mission) and the access administration of the event
  manager, and the ballistics catalog upload.
- [Server infrastructure domain](/crates/api/api_server_infrastructure/src/) — the servers,
  [fleet commands](/documentation/glossary/a_to_f.md#fleet-command),
  [fleet scenarios](/documentation/glossary/a_to_f.md#fleet-scenario) and
  [machine credentials](/documentation/glossary/g_to_m.md#machine-credential) of server control.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the page code, the API handlers it calls and the
  ticket manager (`ttm`), which the feature docs are written from.
- Used by: the glossary's entries for the six older pages and for administration; the in-code READMEs of
  the page folders, which link their feature docs under Related documentation; the API domain
  READMEs that link the feature doc of the page they serve.
- Rules: one folder per page folder of the code, spelled the same; a page's feature doc is named
  after its route component (`personnel_roster_page.md` for `PersonnelRosterPage`) and keeps its
  name, since the glossary and the READMEs link it; a feature doc stays within 500 lines; design
  references live only in `visual_references/`, and no document holds a screenshot of the built UI.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) — the
  route table and the hub that links every page's feature doc.
