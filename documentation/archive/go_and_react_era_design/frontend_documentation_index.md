**Status:** archived — see [frontend documentation](/documentation/apps/frontend/README.md)

# Frontend Documentation Index

Master index for all TBD Reforger frontend surfaces.

**Doc hub:** [docs/website/frontend/ROADMAP.md](/documentation/apps/frontend/README.md) · **Mission Creator:** [ROADMAP.md](/documentation/apps/frontend/workspaces/editor/mission_creator_roadmap.md) · **Tickets:** [TICKET_LEAD.md](https://github.com/darkforce09/TBD-reforger/blob/2574b0ed2f76edb131447eb10e3d55446404d785/docs/TICKET_LEAD.md)

| Doc | Route | Status | Handoff § | Ticket |
|-----|-------|--------|-----------|--------|
| [shell/sidebar.md](/documentation/apps/frontend/shell/app_layout_and_navigation.md) | (shell) | doc-complete | — | — |
| [shell/topnav.md](/documentation/apps/frontend/shell/app_layout_and_navigation.md) | (shell) | doc-complete | — | — |
| [shell/app-layout.md](/documentation/apps/frontend/shell/app_layout_and_navigation.md) | (shell) | doc-complete | — | — |
| [auth/login.md](/documentation/apps/frontend/pages/account/account_pages.md) | `/login` | doc-complete | §2.A | — |
| [auth/auth-callback.md](/documentation/apps/frontend/pages/account/account_pages.md) | `/auth/callback` | doc-complete | — | T-002 (shipped) |
| [pages/dashboard.md](/documentation/apps/frontend/pages/command_center/dashboard/dashboard_page.md) | `/` | doc-complete | — | — |
| [pages/server-intel.md](/documentation/apps/frontend/pages/command_center/server_intel/server_intel_page.md) | `/server-intel` | doc-complete | §4.1 | T-088 |
| [pages/announcements.md](/documentation/apps/frontend/pages/command_center/announcements/announcements_page.md) | `/announcements` | doc-complete | §4.2 | — |
| [pages/deployments.md](/documentation/apps/frontend/pages/operations/deployments/deployments_page.md) | `/deployments` | doc-complete | §4.3 | T-122 ORBAT deep-link |
| [pages/leaderboards.md](/documentation/apps/frontend/pages/operations/leaderboards/leaderboards_page.md) | `/leaderboards` | doc-complete | §4.4 | — |
| [pages/mission-library.md](/documentation/apps/frontend/pages/mission_hub/library/mission_library_page.md) | `/missions` (+ create dialog T-048) | in-progress | §4.5 | — |
| [pages/mission-overview.md](/documentation/apps/frontend/pages/mission_hub/overview/mission_overview_page.md) | `/missions/:id` | doc-complete | — | — |
| [pages/mission-creator.md](/documentation/archive/go_and_react_era_design/mission_creator_setup_wizard_page.md) | *(embedded in `/missions` — T-048)* | archived-route | — | — |
| [pages/mission-editor.md](/documentation/apps/frontend/workspaces/editor/ux_spec.md) | `/missions/:id/edit` | in-progress | — | T-091 + T-122 C3 error overlay; active T-090.3.0 (T-090.1 queued) |
| [pages/event-schedule.md](/documentation/apps/frontend/pages/operations/schedule/event_schedule_page.md) | `/events` | doc-complete | — | — |
| [pages/event-hub.md](/documentation/apps/frontend/pages/operations/event_detail/event_hub_page.md) | `/events/:id` | doc-complete | — | — |
| [pages/modpacks.md](/documentation/apps/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md) | `/modpacks` | doc-complete | §4.7 | — |
| [pages/wiki.md](/documentation/apps/frontend/pages/doctrine_and_info/wiki/wiki_page.md) | `/wiki` | doc-complete | §4.6 | T-085 |
| [pages/vehicle-database.md](/documentation/apps/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md) | `/vehicles` | doc-complete | §4.6 | — |
| [pages/mortar-calculator.md](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md) | `/tools/mortar` | doc-complete | — | — |
| [pages/event-manager.md](/documentation/apps/frontend/pages/administration/event_manager/event_manager_page.md) | `/admin/events` | doc-complete | §4.8 | — |
| [pages/mission-approvals.md](/documentation/apps/frontend/pages/administration/approvals/mission_approvals_page.md) | `/admin/approvals` | doc-complete | §4.9 | — |
| [pages/server-control.md](/documentation/apps/frontend/pages/administration/server_control/server_control_page.md) | `/admin/server` | doc-complete | §2.B | T-086 |
| [pages/personnel-roster.md](/documentation/apps/frontend/pages/administration/personnel/personnel_roster_page.md) | `/admin/personnel` | doc-complete | §4.10 | shipped |
| [pages/content-manager.md](/documentation/apps/frontend/pages/administration/content_manager/content_manager_page.md) | `/admin/content` | doc-complete | §4.11 | T-087 |
| [pages/audit-logs.md](/documentation/apps/frontend/pages/administration/audit_logs/audit_logs_page.md) | `/admin/audit` | doc-complete | §4.12 | shipped |
| [pages/settings.md](/documentation/apps/frontend/pages/account/account_pages.md) | `/settings` | doc-complete | — | — |
| [pages/debug-building-viewer.md](/documentation/apps/frontend/workspaces/debug/building_viewer_page.md) | `/debug/building-viewer` (URL-only) | in-progress | — | blueprint program Phase A |
| [pages/not-found.md](/documentation/apps/frontend/shell/app_layout_and_navigation.md) | `*` | doc-complete | — | — |

**Foundation:** [THEME.md](/documentation/design_system/design_tokens.md) | [TRACKING.md](/documentation/apps/frontend/README.md) | [_template.md](/documentation/archive/go_and_react_era_design/frontend_page_spec_template.md)

**Documentation Gate:** Passed — 29 surface docs + 3 foundation files = 32; all `doc-complete` except [mission-editor.md](/documentation/apps/frontend/workspaces/editor/ux_spec.md) and [debug-building-viewer.md](/documentation/apps/frontend/workspaces/debug/building_viewer_page.md) (`in-progress`).
