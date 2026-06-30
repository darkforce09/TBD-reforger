# Frontend — ROADMAP

**Start here.** Planning view for the React SPA — what is **shipped**, what is **deferred**, and links to every surface doc.

**Queue:** [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) · **Full registry:** [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md)

**Code:** [`apps/website/frontend/src/`](../../../apps/website/frontend/src) · **Routes:** [`apps/website/frontend/src/router.tsx`](../../../apps/website/frontend/src/router.tsx)

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`docs/website/frontend/INDEX.md`](INDEX.md)** | Per-route surface specs (28 pages) |
| **[`docs/website/frontend/THEME.md`](THEME.md)** | Aegis tokens in use |
| **[`docs/website/frontend/_template.md`](_template.md)** | Template for new page docs |
| **[Mission Creator ROADMAP](../../specs/Mission_Creator_Architecture/ROADMAP.md)** | 2D editor ticket queue |
| **[`docs/platform/CODEBASE_AUDIT_2026.md`](../../platform/CODEBASE_AUDIT_2026.md)** | T-122 audit + T-123 resolutions (T1/T8) |
| **[`docs/platform/DOCUMENTATION_STANDARDS.md`](../../platform/DOCUMENTATION_STANDARDS.md)** | Cross-boundary `@contract` / codegen / validation (T-123 shipped) |
| **[`docs/platform/macos_ux_architecture.md`](../platform/macos_ux_architecture.md)** | Split-pane / frictionlessness methodology |
| **[`CLAUDE.md`](../../../apps/website/CLAUDE.md)** | Agent runtime, T-0xx status, doc-on-commit rule |
| **[`docs/AGENT_COMMIT_CHECKLIST.md`](../AGENT_COMMIT_CHECKLIST.md)** | Same-commit doc sync — read before every T-0xx |
| **[Archive](../archive/README.md)** | Historical stitch/blueprint HTML (reference only) |

---

## DONE — shipped surfaces

All routes below have a surface spec unless noted. Live UI = `apps/website/frontend/src/pages` + `features/`.

| Route | Doc | Notes |
|-------|-----|-------|
| `/` | [dashboard.md](pages/dashboard.md) | Glass bento home |
| `/login`, `/auth/callback` | [login.md](auth/login.md), [auth-callback.md](auth/auth-callback.md) | Discord OAuth + dev-login |
| `/server-intel` | [server-intel.md](pages/server-intel.md) | |
| `/announcements` | [announcements.md](pages/announcements.md) | Live: `operations.tsx` |
| `/deployments` | [deployments.md](pages/deployments.md) | Live: `operations.tsx`; **T-122** ORBAT deep-link from Modify Assignment |
| `/leaderboards` | [leaderboards.md](pages/leaderboards.md) | Live: `operations.tsx` |
| `/missions` | [mission-library.md](pages/mission-library.md) | Create dialog shipped (T-048); `/missions/create` removed |
| `/missions/:id` | [mission-overview.md](pages/mission-overview.md) | Sheet dossier |
| `/missions/:id/edit` | [mission-editor.md](pages/mission-editor.md) | **in-progress** — T-091 shipped @ `dde589e` (DEM + Z + hillshade); **T-090.3.0** Workbench spike active (**T-090.1** aligned tiles queued) |
| `/events` | [event-schedule.md](pages/event-schedule.md) | SplitPane; Live: `operations.tsx` |
| `/events/:id` | [event-hub.md](pages/event-hub.md) | Inline ORBAT |
| `/events/:id/missions/:emid/orbat` | [event-hub.md § ORBAT deep-link](pages/event-hub.md) | Wired from Deployments (T-122 R2) |
| `/wiki`, `/wiki/:slug` | [wiki.md](pages/wiki.md) | Doctrine SOPs |
| `/vehicles` | [vehicle-database.md](pages/vehicle-database.md) | Split from wiki |
| `/modpacks` | [modpacks.md](pages/modpacks.md) | |
| `/tools/mortar` | [mortar-calculator.md](pages/mortar-calculator.md) | |
| `/settings` | [settings.md](pages/settings.md) | |
| `/admin/events` | [event-manager.md](pages/event-manager.md) | |
| `/admin/approvals` | [mission-approvals.md](pages/mission-approvals.md) | |
| `/admin/server` | [server-control.md](pages/server-control.md) | **stub** — **T-086** |
| `/admin/personnel` | [personnel-roster.md](pages/personnel-roster.md) | Live API |
| `/admin/content` | [content-manager.md](pages/content-manager.md) | Nav: Comms Broadcaster |
| `/admin/audit` | [audit-logs.md](pages/audit-logs.md) | Live API |
| `*` | [not-found.md](pages/not-found.md) | |
| (shell) | [sidebar.md](shell/sidebar.md), [topnav.md](shell/topnav.md), [app-layout.md](shell/app-layout.md) | |

---

## NOT DONE — deferred (T-IDs)

| T-ID | Item | Doc | Blocked by |
|------|------|-----|------------|
| **T-085** | Wiki markdown renderer | [wiki.md](pages/wiki.md) | react-markdown |
| **T-086** | Server Control `/admin/server` | [server-control.md](pages/server-control.md) | **T-086** backend RCON API |
| **T-087** | CMS rich text | [content-manager.md](pages/content-manager.md) | WYSIWYG choice |
| **T-088** | Multi-server picker | [server-intel.md](pages/server-intel.md) | UI for `GET /servers` |
| **T-068+** | Mission editor Eden parity | [mission-editor.md](pages/mission-editor.md) | **T-068 Phase 1 shipped**; Phase 2 paused; **T-090–T-092** map gate active |

Full deferred table: [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md).

---

## Recently shipped

| Item | Spec | Notes |
|------|------|-------|
| **T-068.5 mod equip (shipped `21ec91e`)** | [t068_5_mod_equip_loadout.md](../../specs/Mission_Creator_Architecture/t068_5_mod_equip_loadout.md) | `TBD_LoadoutEquipComponent` — profile JSON → equip @ 6400 |
| **T-068.4 dumb loadout UI (shipped `a85f16b`)** | [t068_4_dumb_loadout_ui.md](../../specs/Mission_Creator_Architecture/t068_4_dumb_loadout_ui.md) | Arsenal tab — 4 gear dropdowns + `loadout-export.json` download |
| **T-068.3 palette wire (shipped `da78452`)** | [t068_3_palette_wire.md](../../specs/Mission_Creator_Architecture/t068_3_palette_wire.md) | `useRegistry` + `buildCatalogTree`; mock deleted; `resource_name` on DnD |
| **T-068.2 registry API (shipped `4c609fe`)** | [t068_2_registry_api.md](../../specs/Mission_Creator_Architecture/t068_2_registry_api.md) | `GET /api/v1/registry`, dev seed, import CLI |
| **T-061 drag-move @ 360k (shipped — good enough)** | [t061_drag_move_hotfix.md](../../specs/Mission_Creator_Architecture/t061_drag_move_hotfix.md) | T-061.0 motion ~60 fps + T-061.0.1 `slotIconCache` + slot fast path; **T-094** deferred |
| **T-062 incremental bindings (shipped)** | [t062_incremental_bindings.md](../../specs/Mission_Creator_Architecture/t062_incremental_bindings.md) | Classifier + bulk delete @ 360k |
| **T-062.2 editor session (shipped)** | [t062_2_editor_session_persistence.md](../../specs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md) | Alt-tab / warm session fast path |
| **T-060 scale load/save (shipped `b1fd25a`)** | [t060_1](../../specs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) · [t060](../../specs/Mission_Creator_Architecture/t060_fast_initial_load.md) | Four-phase load; Save @ ~367k/~142 MB → 201 |
| **T-064 Virtualized outliner (shipped)** | [t064_virtualized_outliner.md](../../specs/Mission_Creator_Architecture/t064_virtualized_outliner.md) | `@tanstack/react-virtual` + segment flatten; scrollable @ ~367k; T-064.1 scroll-ref hotfix |
| **T-063 Spatial index (shipped)** | [t063_spatial_index.md](../../specs/Mission_Creator_Architecture/t063_spatial_index.md) | rbush pick/marquee @ ~367k |
| **T-059 Bulk paste at scale** | [t059_bulk_paste_operations.md](../../specs/Mission_Creator_Architecture/t059_bulk_paste_operations.md) | Batch O(n) `pasteSlots`; selection cap 500; outliner virtualization (T-064). **Validated: 360k @ 100+ fps** pan; 6k paste loops smooth |
| **T-058 Toolbelt OBJ/SEL counts** | [t058_entity_count_readout.md](../../specs/Mission_Creator_Architecture/t058_entity_count_readout.md) | OBJ + SEL in toolbelt; scale telemetry |
| **T-057 Map perf hotfix** | [t057_map_performance_hotfix.md](../../specs/Mission_Creator_Architecture/t057_map_performance_hotfix.md) | ≥55 fps pan/zoom @ 200+ slots |
| **T-056 Ctrl+C/V copy-paste** | [t056_copy_paste.md](../../specs/Mission_Creator_Architecture/t056_copy_paste.md) | Copy/paste at cursor; one undo step |
| **T-055 Asset browser search** | [t055_asset_browser_search.md](../../specs/Mission_Creator_Architecture/t055_asset_browser_search.md) | Factions tree filter; X/Esc clears |
| **T-054 Attributes entry points** | [t054_attributes_entry_points.md](../../specs/Mission_Creator_Architecture/t054_attributes_entry_points.md) | Map + ORBAT dbl-click → Attributes |
| **T-053 Ctrl/Cmd additive select** | [t053_additive_select.md](../../specs/Mission_Creator_Architecture/t053_additive_select.md) | Modifier-click toggle select |
| **T-052 Undo/redo keyboard** | [t052_undo_shortcuts.md](../../specs/Mission_Creator_Architecture/t052_undo_shortcuts.md) | Keyboard undo/redo + StrictMode fix |
| **T-050 Cursor Z readout** | [t050_cursor_z_readout.md](../../specs/Mission_Creator_Architecture/t050_cursor_z_readout.md) | Toolbelt CUR X/Y/Z until **T-091** DEM |
| **T-049 Terrain, title, position** | [t049_terrain_title_position.md](../../specs/Mission_Creator_Architecture/t049_terrain_title_position.md) | Terrain viewport; row meta hydrate; editable transform |
| **T-048 Library create dialog** | [t048_library_create_dialog.md](../../specs/Mission_Creator_Architecture/t048_library_create_dialog.md) | `CreateMissionDialog` on `/missions` |

## Recommended next work

1. **T-068.6** — human Phase 1 E2E sign-off ([`t068_6_phase1_e2e_gate.md`](../../specs/Mission_Creator_Architecture/t068_6_phase1_e2e_gate.md)) — all automated slices shipped through T-068.5
2. **T-085** — wiki markdown (low risk, high UX)
3. **T-086** — when backend exposes server/RCON endpoints

---

## Design system

- **Live tokens:** [`apps/website/frontend/src/index.css`](../../../apps/website/frontend/src/index.css)
- **Reference YAML:** [`docs/specs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md`](../../specs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md)
- **Methodology:** [`docs/platform/macos_ux_architecture.md`](../platform/macos_ux_architecture.md)

Do not implement from archived stitch `code.html`.
