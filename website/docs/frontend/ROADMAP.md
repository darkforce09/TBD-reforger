# Frontend — ROADMAP

**Start here.** Planning view for the React SPA — what is **shipped**, what is **deferred**, and links to every surface doc.

**Queue:** [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) · **Full registry:** [`docs/TICKET_REGISTRY.md`](../TICKET_REGISTRY.md)

**Code:** [`frontend/src/`](../../frontend/src/) · **Routes:** [`frontend/src/router.tsx`](../../frontend/src/router.tsx)

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`frontend/docs/INDEX.md`](../../frontend/docs/INDEX.md)** | Per-route surface specs (28 pages) |
| **[`frontend/docs/THEME.md`](../../frontend/docs/THEME.md)** | Aegis tokens in use |
| **[`frontend/docs/_template.md`](../../frontend/docs/_template.md)** | Template for new page docs |
| **[Mission Creator ROADMAP](../../Design_Docs/Mission_Creator_Architecture/ROADMAP.md)** | 2D editor ticket queue |
| **[`docs/platform/macos_ux_architecture.md`](../platform/macos_ux_architecture.md)** | Split-pane / frictionlessness methodology |
| **[`CLAUDE.md`](../../CLAUDE.md)** | Agent runtime, T-0xx status, doc-on-commit rule |
| **[`docs/AGENT_COMMIT_CHECKLIST.md`](../AGENT_COMMIT_CHECKLIST.md)** | Same-commit doc sync — read before every T-0xx |
| **[Archive](../archive/README.md)** | Historical stitch/blueprint HTML (reference only) |

---

## DONE — shipped surfaces

All routes below have a surface spec unless noted. Live UI = `frontend/src/pages` + `features/`.

| Route | Doc | Notes |
|-------|-----|-------|
| `/` | [dashboard.md](../../frontend/docs/pages/dashboard.md) | Glass bento home |
| `/login`, `/auth/callback` | [login.md](../../frontend/docs/auth/login.md), [auth-callback.md](../../frontend/docs/auth/auth-callback.md) | Discord OAuth + dev-login |
| `/server-intel` | [server-intel.md](../../frontend/docs/pages/server-intel.md) | |
| `/announcements` | [announcements.md](../../frontend/docs/pages/announcements.md) | Live: `operations.tsx` |
| `/deployments` | [deployments.md](../../frontend/docs/pages/deployments.md) | Live: `operations.tsx` |
| `/leaderboards` | [leaderboards.md](../../frontend/docs/pages/leaderboards.md) | Live: `operations.tsx` |
| `/missions` | [mission-library.md](../../frontend/docs/pages/mission-library.md) | Create dialog shipped (T-048); `/missions/create` removed |
| `/missions/:id` | [mission-overview.md](../../frontend/docs/pages/mission-overview.md) | Sheet dossier |
| `/missions/:id/edit` | [mission-editor.md](../../frontend/docs/pages/mission-editor.md) | **in-progress** — T-067 shipped ([`t067_spatial_chunks.md`](../../Design_Docs/Mission_Creator_Architecture/t067_spatial_chunks.md)); next T-068+ |
| `/events` | [event-schedule.md](../../frontend/docs/pages/event-schedule.md) | SplitPane; Live: `operations.tsx` |
| `/events/:id` | [event-hub.md](../../frontend/docs/pages/event-hub.md) | Inline ORBAT |
| `/events/:id/missions/:emid/orbat` | [event-hub.md § ORBAT deep-link](../../frontend/docs/pages/event-hub.md) | |
| `/wiki`, `/wiki/:slug` | [wiki.md](../../frontend/docs/pages/wiki.md) | Doctrine SOPs |
| `/vehicles` | [vehicle-database.md](../../frontend/docs/pages/vehicle-database.md) | Split from wiki |
| `/modpacks` | [modpacks.md](../../frontend/docs/pages/modpacks.md) | |
| `/tools/mortar` | [mortar-calculator.md](../../frontend/docs/pages/mortar-calculator.md) | |
| `/settings` | [settings.md](../../frontend/docs/pages/settings.md) | |
| `/admin/events` | [event-manager.md](../../frontend/docs/pages/event-manager.md) | |
| `/admin/approvals` | [mission-approvals.md](../../frontend/docs/pages/mission-approvals.md) | |
| `/admin/server` | [server-control.md](../../frontend/docs/pages/server-control.md) | **stub** — **T-086** |
| `/admin/personnel` | [personnel-roster.md](../../frontend/docs/pages/personnel-roster.md) | Live API |
| `/admin/content` | [content-manager.md](../../frontend/docs/pages/content-manager.md) | Nav: Comms Broadcaster |
| `/admin/audit` | [audit-logs.md](../../frontend/docs/pages/audit-logs.md) | Live API |
| `*` | [not-found.md](../../frontend/docs/pages/not-found.md) | |
| (shell) | [sidebar.md](../../frontend/docs/shell/sidebar.md), [topnav.md](../../frontend/docs/shell/topnav.md), [app-layout.md](../../frontend/docs/shell/app-layout.md) | |

---

## NOT DONE — deferred (T-IDs)

| T-ID | Item | Doc | Blocked by |
|------|------|-----|------------|
| **T-085** | Wiki markdown renderer | [wiki.md](../../frontend/docs/pages/wiki.md) | react-markdown |
| **T-086** | Server Control `/admin/server` | [server-control.md](../../frontend/docs/pages/server-control.md) | **T-086** backend RCON API |
| **T-087** | CMS rich text | [content-manager.md](../../frontend/docs/pages/content-manager.md) | WYSIWYG choice |
| **T-088** | Multi-server picker | [server-intel.md](../../frontend/docs/pages/server-intel.md) | UI for `GET /servers` |
| **T-068+** | Mission editor Eden parity | [mission-editor.md](../../frontend/docs/pages/mission-editor.md) | Scale **T-067** then registry (**T-068**), markers (**T-069**), … |

Full deferred table: [`docs/TICKET_REGISTRY.md`](../TICKET_REGISTRY.md).

---

## Recently shipped

| Item | Spec | Notes |
|------|------|-------|
| **T-061 drag-move @ 360k (shipped — good enough)** | [t061_drag_move_hotfix.md](../../Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md) | T-061.0 motion ~60 fps + T-061.0.1 `slotIconCache` + slot fast path; **T-094** deferred |
| **T-062 incremental bindings (shipped)** | [t062_incremental_bindings.md](../../Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md) | Classifier + bulk delete @ 360k |
| **T-062.2 editor session (shipped)** | [t062_2_editor_session_persistence.md](../../Design_Docs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md) | Alt-tab / warm session fast path |
| **T-060 scale load/save (shipped `b1fd25a`)** | [t060_1](../../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) · [t060](../../Design_Docs/Mission_Creator_Architecture/t060_fast_initial_load.md) | Four-phase load; Save @ ~367k/~142 MB → 201 |
| **T-064 Virtualized outliner (shipped)** | [t064_virtualized_outliner.md](../../Design_Docs/Mission_Creator_Architecture/t064_virtualized_outliner.md) | `@tanstack/react-virtual` + segment flatten; scrollable @ ~367k; T-064.1 scroll-ref hotfix |
| **T-063 Spatial index (shipped)** | [t063_spatial_index.md](../../Design_Docs/Mission_Creator_Architecture/t063_spatial_index.md) | rbush pick/marquee @ ~367k |
| **T-059 Bulk paste at scale** | [t059_bulk_paste_operations.md](../../Design_Docs/Mission_Creator_Architecture/t059_bulk_paste_operations.md) | Batch O(n) `pasteSlots`; selection cap 500; outliner virtualization (T-064). **Validated: 360k @ 100+ fps** pan; 6k paste loops smooth |
| **T-058 Toolbelt OBJ/SEL counts** | [t058_entity_count_readout.md](../../Design_Docs/Mission_Creator_Architecture/t058_entity_count_readout.md) | OBJ + SEL in toolbelt; scale telemetry |
| **T-057 Map perf hotfix** | [t057_map_performance_hotfix.md](../../Design_Docs/Mission_Creator_Architecture/t057_map_performance_hotfix.md) | ≥55 fps pan/zoom @ 200+ slots |
| **T-056 Ctrl+C/V copy-paste** | [t056_copy_paste.md](../../Design_Docs/Mission_Creator_Architecture/t056_copy_paste.md) | Copy/paste at cursor; one undo step |
| **T-055 Asset browser search** | [t055_asset_browser_search.md](../../Design_Docs/Mission_Creator_Architecture/t055_asset_browser_search.md) | Factions tree filter; X/Esc clears |
| **T-054 Attributes entry points** | [t054_attributes_entry_points.md](../../Design_Docs/Mission_Creator_Architecture/t054_attributes_entry_points.md) | Map + ORBAT dbl-click → Attributes |
| **T-053 Ctrl/Cmd additive select** | [t053_additive_select.md](../../Design_Docs/Mission_Creator_Architecture/t053_additive_select.md) | Modifier-click toggle select |
| **T-052 Undo/redo keyboard** | [t052_undo_shortcuts.md](../../Design_Docs/Mission_Creator_Architecture/t052_undo_shortcuts.md) | Keyboard undo/redo + StrictMode fix |
| **T-050 Cursor Z readout** | [t050_cursor_z_readout.md](../../Design_Docs/Mission_Creator_Architecture/t050_cursor_z_readout.md) | Toolbelt CUR X/Y/Z until **T-091** DEM |
| **T-049 Terrain, title, position** | [t049_terrain_title_position.md](../../Design_Docs/Mission_Creator_Architecture/t049_terrain_title_position.md) | Terrain viewport; row meta hydrate; editable transform |
| **T-048 Library create dialog** | [t048_library_create_dialog.md](../../Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md) | `CreateMissionDialog` on `/missions` |

## Recommended next work

1. **Mission Creator — Eden T-068+** per [`TICKET_LEAD.md`](../TICKET_LEAD.md) (T-067 scale program shipped)
2. **T-085** — wiki markdown (low risk, high UX)
3. **T-086** — when backend exposes server/RCON endpoints

---

## Design system

- **Live tokens:** [`frontend/src/index.css`](../../frontend/src/index.css)
- **Reference YAML:** [`Design_Docs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md`](../../Design_Docs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md)
- **Methodology:** [`docs/platform/macos_ux_architecture.md`](../platform/macos_ux_architecture.md)

Do not implement from archived stitch `code.html`.
