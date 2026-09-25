**Status:** live

# Frontend — ROADMAP

**Start here.** Planning view for the Leptos SPA — what is **shipped**, what is **deferred**, and links to every surface doc.

**Queue:** [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) · **Full registry:** [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md)

**Code:** [`apps/website/frontend/src/`](../../../apps/website/frontend/src) · **Routes:** [`apps/website/frontend/src/router.rs`](../../../apps/website/frontend/src/router.rs) · Conventions: [`WHERE_DOES_X_GO.md`](/documentation_v2/standards/where_does_x_go.md)

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`documentation_v2/website/frontend/README.md`](/documentation_v2/website/frontend/README.md)** | Per-route surface specs (28 pages) |
| **[`documentation_v2/design_system/design_tokens.md`](/documentation_v2/design_system/design_tokens.md)** | Aegis tokens in use |
| **[`documentation_v2/archive/go_and_react_era_design/frontend_page_spec_template.md`](/documentation_v2/archive/go_and_react_era_design/frontend_page_spec_template.md)** | Template for new page docs |
| **[Mission Creator ROADMAP](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)** | 2D editor ticket queue |
| **[`documentation_v2/archive/audits/codebase_audit_2026.md`](/documentation_v2/archive/audits/codebase_audit_2026.md)** | T-122 audit + T-123 resolutions (T1/T8) |
| **[`documentation_v2/standards/documentation_standards.md`](/documentation_v2/standards/documentation_standards.md)** | Cross-boundary `@contract` / codegen / validation (T-123 shipped) |
| **[`docs/platform/macos_ux_architecture.md`](/documentation_v2/archive/go_and_react_era_design/macos_ux_architecture.md)** | Split-pane / frictionlessness methodology |
| **Root [`CLAUDE.md`](../../../CLAUDE.md)** | Agent runtime, T-0xx status, doc-on-commit rule |
| **[`docs/AGENT_COMMIT_CHECKLIST.md`](/documentation_v2/standards/commit_checklist.md)** | Same-commit doc sync — read before every T-0xx |
| **[Archive](/documentation_v2/archive/monorepo_migration/docs_website_archive_readme.md)** | Historical stitch/blueprint HTML (reference only) |

---

## DONE — shipped surfaces

All routes below have a surface spec unless noted. Live UI = `apps/website/frontend/src/` (T-159).

| Route | Doc | Notes |
|-------|-----|-------|
| `/` | [dashboard.md](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md) | Glass bento home |
| `/login`, `/auth/callback` | [login.md](/documentation_v2/website/frontend/pages/account/account_pages.md), [auth-callback.md](/documentation_v2/website/frontend/pages/account/account_pages.md) | Discord OAuth + dev-login |
| `/server-intel` | [server-intel.md](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md) | |
| `/announcements` | [announcements.md](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md) | Live: `operations.tsx` |
| `/deployments` | [deployments.md](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md) | Live: `operations.tsx`; **T-122** ORBAT deep-link from Modify Assignment |
| `/leaderboards` | [leaderboards.md](/documentation_v2/website/frontend/pages/operations/leaderboards/leaderboards_page.md) | Live: `operations.tsx` |
| `/missions` | [mission-library.md](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md) | Create dialog shipped (T-048); `/missions/create` removed |
| `/missions/:id` | [mission-overview.md](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md) | Sheet dossier |
| `/missions/:id/edit` | [mission-editor.md](/documentation_v2/website/frontend/apps/editor/ux_spec.md) | **in-progress** — T-091 shipped @ `dde589e` (DEM + Z + hillshade); **T-090.3.0** Workbench spike active (**T-090.1** aligned tiles queued) |
| `/events` | [event-schedule.md](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md) | SplitPane; Live: `operations.tsx` |
| `/events/:id` | [event-hub.md](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md) | Inline ORBAT |
| `/events/:id/missions/:emid/orbat` | [event-hub.md § ORBAT deep-link](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md) | Wired from Deployments (T-122 R2) |
| `/wiki`, `/wiki/:slug` | [wiki.md](/documentation_v2/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md) | Doctrine SOPs |
| `/vehicles` | [vehicle-database.md](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md) | Split from wiki |
| `/modpacks` | [modpacks.md](/documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md) | |
| `/tools/mortar` | [mortar-calculator.md](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md) | |
| `/settings` | [settings.md](/documentation_v2/website/frontend/pages/account/account_pages.md) | |
| `/admin/events` | [event-manager.md](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md) | |
| `/admin/approvals` | [mission-approvals.md](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md) | |
| `/admin/server` | [server-control.md](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md) | **stub** — **T-086** |
| `/admin/personnel` | [personnel-roster.md](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md) | Live API |
| `/admin/content` | [content-manager.md](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md) | Nav: Comms Broadcaster |
| `/admin/audit` | [audit-logs.md](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md) | Live API |
| `*` | [not-found.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md) | |
| (shell) | [sidebar.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md), [topnav.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md), [app-layout.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md) | |

---

## NOT DONE — deferred (T-IDs)

| T-ID | Item | Doc | Blocked by |
|------|------|-----|------------|
| **T-085** | Wiki markdown renderer | [wiki.md](/documentation_v2/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md) | react-markdown |
| **T-086** | Server Control `/admin/server` | [server-control.md](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md) | **T-086** backend RCON API |
| **T-087** | CMS rich text | [content-manager.md](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md) | WYSIWYG choice |
| **T-088** | Multi-server picker | [server-intel.md](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md) | UI for `GET /servers` |
| **T-068+** | Mission editor Eden parity | [mission-editor.md](/documentation_v2/website/frontend/apps/editor/ux_spec.md) | **T-068 Phase 1 shipped**; Phase 2 paused; **T-090–T-092** map gate active |

Full deferred table: [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md).

---

## Recently shipped

| Item | Spec | Notes |
|------|------|-------|
| **T-068.5 mod equip (shipped `21ec91e`)** | [t068_5_mod_equip_loadout.md](/documentation_v2/tickets/specs/t068_5_mod_equip_loadout.md) | `TBD_LoadoutEquipComponent` — profile JSON → equip @ 6400 |
| **T-068.4 dumb loadout UI (shipped `a85f16b`)** | [t068_4_dumb_loadout_ui.md](/documentation_v2/tickets/specs/t068_4_dumb_loadout_ui.md) | Arsenal tab — 4 gear dropdowns + `loadout-export.json` download |
| **T-068.3 palette wire (shipped `da78452`)** | [t068_3_palette_wire.md](/documentation_v2/tickets/specs/t068_3_palette_wire.md) | `useRegistry` + `buildCatalogTree`; mock deleted; `resource_name` on DnD |
| **T-068.2 registry API (shipped `4c609fe`)** | [t068_2_registry_api.md](/documentation_v2/tickets/specs/t068_2_registry_api.md) | `GET /api/v1/registry`, dev seed, import CLI |
| **T-061 drag-move @ 360k (shipped — good enough)** | [t061_drag_move_hotfix.md](/documentation_v2/tickets/specs/t061_drag_move_hotfix.md) | T-061.0 motion ~60 fps + T-061.0.1 `slotIconCache` + slot fast path; **T-094** deferred |
| **T-062 incremental bindings (shipped)** | [t062_incremental_bindings.md](/documentation_v2/tickets/specs/t062_incremental_bindings.md) | Classifier + bulk delete @ 360k |
| **T-062.2 editor session (shipped)** | [t062_2_editor_session_persistence.md](/documentation_v2/tickets/specs/t062_2_editor_session_persistence.md) | Alt-tab / warm session fast path |
| **T-060 scale load/save (shipped `b1fd25a`)** | [t060_1](/documentation_v2/tickets/specs/t060_1_scale_load_save_completion.md) · [t060](/documentation_v2/tickets/specs/t060_fast_initial_load.md) | Four-phase load; Save @ ~367k/~142 MB → 201 |
| **T-064 Virtualized outliner (shipped)** | [t064_virtualized_outliner.md](/documentation_v2/tickets/specs/t064_virtualized_outliner.md) | `@tanstack/react-virtual` + segment flatten; scrollable @ ~367k; T-064.1 scroll-ref hotfix |
| **T-063 Spatial index (shipped)** | [t063_spatial_index.md](/documentation_v2/tickets/specs/t063_spatial_index.md) | rbush pick/marquee @ ~367k |
| **T-059 Bulk paste at scale** | [t059_bulk_paste_operations.md](/documentation_v2/tickets/specs/t059_bulk_paste_operations.md) | Batch O(n) `pasteSlots`; selection cap 500; outliner virtualization (T-064). **Validated: 360k @ 100+ fps** pan; 6k paste loops smooth |
| **T-058 Toolbelt OBJ/SEL counts** | [t058_entity_count_readout.md](/documentation_v2/tickets/specs/t058_entity_count_readout.md) | OBJ + SEL in toolbelt; scale telemetry |
| **T-057 Map perf hotfix** | [t057_map_performance_hotfix.md](/documentation_v2/tickets/specs/t057_map_performance_hotfix.md) | ≥55 fps pan/zoom @ 200+ slots |
| **T-056 Ctrl+C/V copy-paste** | [t056_copy_paste.md](/documentation_v2/tickets/specs/t056_copy_paste.md) | Copy/paste at cursor; one undo step |
| **T-055 Asset browser search** | [t055_asset_browser_search.md](/documentation_v2/tickets/specs/t055_asset_browser_search.md) | Factions tree filter; X/Esc clears |
| **T-054 Attributes entry points** | [t054_attributes_entry_points.md](/documentation_v2/tickets/specs/t054_attributes_entry_points.md) | Map + ORBAT dbl-click → Attributes |
| **T-053 Ctrl/Cmd additive select** | [t053_additive_select.md](/documentation_v2/tickets/specs/t053_additive_select.md) | Modifier-click toggle select |
| **T-052 Undo/redo keyboard** | [t052_undo_shortcuts.md](/documentation_v2/tickets/specs/t052_undo_shortcuts.md) | Keyboard undo/redo + StrictMode fix |
| **T-050 Cursor Z readout** | [t050_cursor_z_readout.md](/documentation_v2/tickets/specs/t050_cursor_z_readout.md) | Toolbelt CUR X/Y/Z until **T-091** DEM |
| **T-049 Terrain, title, position** | [t049_terrain_title_position.md](/documentation_v2/tickets/specs/t049_terrain_title_position.md) | Terrain viewport; row meta hydrate; editable transform |
| **T-048 Library create dialog** | [t048_library_create_dialog.md](/documentation_v2/tickets/specs/t048_library_create_dialog.md) | `CreateMissionDialog` on `/missions` |

## Recommended next work

1. **T-068.6** — human Phase 1 E2E sign-off ([`t068_6_phase1_e2e_gate.md`](/documentation_v2/tickets/specs/t068_6_phase1_e2e_gate.md)) — all automated slices shipped through T-068.5
2. **T-085** — wiki markdown (low risk, high UX)
3. **T-086** — when backend exposes server/RCON endpoints

---

## Design system

- **Live tokens:** [`apps/website/frontend/style/aegis.css`](../../../apps/website/frontend/style/aegis.css)
- **Reference YAML:** [`documentation_v2/design_system/token_exports/aegis_design_tokens.md`](/documentation_v2/design_system/token_exports/aegis_design_tokens.md)
- **Methodology:** [`docs/platform/macos_ux_architecture.md`](/documentation_v2/archive/go_and_react_era_design/macos_ux_architecture.md)

Do not implement from archived stitch `code.html`.
