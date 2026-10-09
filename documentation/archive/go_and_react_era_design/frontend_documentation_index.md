**Status:** archived — see [frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md)

# Frontend Documentation Index

Master index for all TBD Reforger frontend surfaces.

**Doc hub:** [docs/website/frontend/ROADMAP.md](/documentation/crates/frontend/shell/frontend_application/README.md) · **Mission Creator:** [ROADMAP.md](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md) · **Tickets:** [TICKET_LEAD.md](https://github.com/darkforce09/TBD-reforger/blob/2574b0ed2f76edb131447eb10e3d55446404d785/docs/TICKET_LEAD.md)

| Doc | Route | Status | Handoff § | Ticket |
|-----|-------|--------|-----------|--------|
| [shell/sidebar.md](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md) | (shell) | doc-complete | — | — |
| [shell/topnav.md](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md) | (shell) | doc-complete | — | — |
| [shell/app-layout.md](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md) | (shell) | doc-complete | — | — |
| [auth/login.md](/documentation/crates/frontend/pages/account_pages/account_pages.md) | `/login` | doc-complete | §2.A | — |
| [auth/auth-callback.md](/documentation/crates/frontend/pages/account_pages/account_pages.md) | `/auth/callback` | doc-complete | — | T-002 (shipped) |
| [pages/dashboard.md](/documentation/crates/frontend/pages/command_center_pages/dashboard/dashboard_page.md) | `/` | doc-complete | — | — |
| [pages/server-intel.md](/documentation/crates/frontend/pages/command_center_pages/server_intel/server_intel_page.md) | `/server-intel` | doc-complete | §4.1 | T-088 |
| [pages/announcements.md](/documentation/crates/frontend/pages/command_center_pages/announcements/announcements_page.md) | `/announcements` | doc-complete | §4.2 | — |
| [pages/deployments.md](/documentation/crates/frontend/pages/operations_pages/deployments/deployments_page.md) | `/deployments` | doc-complete | §4.3 | T-122 ORBAT deep-link |
| [pages/leaderboards.md](/documentation/crates/frontend/pages/operations_pages/leaderboards/leaderboards_page.md) | `/leaderboards` | doc-complete | §4.4 | — |
| [pages/mission-library.md](/documentation/crates/frontend/pages/mission_hub_pages/library/mission_library_page.md) | `/missions` (+ create dialog T-048) | in-progress | §4.5 | — |
| [pages/mission-overview.md](/documentation/crates/frontend/pages/mission_hub_pages/overview/mission_overview_page.md) | `/missions/:id` | doc-complete | — | — |
| [pages/mission-creator.md](/documentation/archive/go_and_react_era_design/mission_creator_setup_wizard_page.md) | *(embedded in `/missions` — T-048)* | archived-route | — | — |
| [pages/mission-editor.md](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) | `/missions/:id/edit` | in-progress | — | T-091 + T-122 C3 error overlay; active T-090.3.0 (T-090.1 queued) |
| [pages/event-schedule.md](/documentation/crates/frontend/pages/operations_pages/schedule/event_schedule_page.md) | `/events` | doc-complete | — | — |
| [pages/event-hub.md](/documentation/crates/frontend/pages/operations_pages/event_detail/event_hub_page.md) | `/events/:id` | doc-complete | — | — |
| [pages/modpacks.md](/documentation/crates/frontend/pages/doctrine_pages/modpacks/modpacks_page.md) | `/modpacks` | doc-complete | §4.7 | — |
| [pages/wiki.md](/documentation/crates/frontend/pages/doctrine_pages/wiki/wiki_page.md) | `/wiki` | doc-complete | §4.6 | T-085 |
| [pages/vehicle-database.md](/documentation/crates/frontend/pages/doctrine_pages/vehicles/vehicle_database_page.md) | `/vehicles` | doc-complete | §4.6 | — |
| [pages/mortar-calculator.md](/documentation/crates/frontend/pages/field_tools_pages/mortar/mortar_calculator_page.md) | `/tools/mortar` | doc-complete | — | — |
| [pages/event-manager.md](/documentation/crates/frontend/pages/administration_pages/event_manager/event_manager_page.md) | `/admin/events` | doc-complete | §4.8 | — |
| [pages/mission-approvals.md](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md) | `/admin/approvals` | doc-complete | §4.9 | — |
| [pages/server-control.md](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md) | `/admin/server` | doc-complete | §2.B | T-086 |
| [pages/personnel-roster.md](/documentation/crates/frontend/pages/administration_pages/personnel/personnel_roster_page.md) | `/admin/personnel` | doc-complete | §4.10 | shipped |
| [pages/content-manager.md](/documentation/crates/frontend/pages/administration_pages/content_manager/content_manager_page.md) | `/admin/content` | doc-complete | §4.11 | T-087 |
| [pages/audit-logs.md](/documentation/crates/frontend/pages/administration_pages/audit_logs/audit_logs_page.md) | `/admin/audit` | doc-complete | §4.12 | shipped |
| [pages/settings.md](/documentation/crates/frontend/pages/account_pages/account_pages.md) | `/settings` | doc-complete | — | — |
| [pages/debug-building-viewer.md](/documentation/crates/frontend/workspaces/debug_benches/building_viewer_page.md) | `/debug/building-viewer` (URL-only) | in-progress | — | blueprint program Phase A |
| [pages/not-found.md](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md) | `*` | doc-complete | — | — |

**Foundation:** [THEME.md](/documentation/design_system/design_tokens.md) | [TRACKING.md](/documentation/crates/frontend/shell/frontend_application/README.md) | [_template.md](/documentation/archive/go_and_react_era_design/frontend_page_spec_template.md)

**Documentation Gate:** Passed — 29 surface docs + 3 foundation files = 32; all `doc-complete` except [mission-editor.md](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) and [debug-building-viewer.md](/documentation/crates/frontend/workspaces/debug_benches/building_viewer_page.md) (`in-progress`).
