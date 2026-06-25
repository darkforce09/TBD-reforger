# Frontend — ROADMAP

**Start here.** Planning view for the React SPA — what is **shipped**, what is **deferred**, and links to every surface doc.

**Code:** [`frontend/src/`](../../frontend/src/) · **Routes:** [`frontend/src/router.tsx`](../../frontend/src/router.tsx)

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`frontend/docs/INDEX.md`](../../frontend/docs/INDEX.md)** | Per-route surface specs (28 pages) |
| **[`frontend/docs/TRACKING.md`](../../frontend/docs/TRACKING.md)** | FD-0xx deferred backlog |
| **[`frontend/docs/THEME.md`](../../frontend/docs/THEME.md)** | Aegis tokens in use |
| **[`frontend/docs/_template.md`](../../frontend/docs/_template.md)** | Template for new page docs |
| **[Mission Creator ROADMAP](../../Design_Docs/Mission_Creator_Architecture/ROADMAP.md)** | 2D editor Tracks A/B/C |
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
| `/missions/:id/edit` | [mission-editor.md](../../frontend/docs/pages/mission-editor.md) | **in-progress** — T-065..T-067 scale active; Eden T-068+ |
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
| `/admin/server` | [server-control.md](../../frontend/docs/pages/server-control.md) | **stub** — FD-001 |
| `/admin/personnel` | [personnel-roster.md](../../frontend/docs/pages/personnel-roster.md) | Live API |
| `/admin/content` | [content-manager.md](../../frontend/docs/pages/content-manager.md) | Nav: Comms Broadcaster |
| `/admin/audit` | [audit-logs.md](../../frontend/docs/pages/audit-logs.md) | Live API |
| `*` | [not-found.md](../../frontend/docs/pages/not-found.md) | |
| (shell) | [sidebar.md](../../frontend/docs/shell/sidebar.md), [topnav.md](../../frontend/docs/shell/topnav.md), [app-layout.md](../../frontend/docs/shell/app-layout.md) | |

---

## NOT DONE — deferred (FD-0xx)

| ID | Item | Doc | Blocked by |
|----|------|-----|------------|
| FD-001 | Server Control `/admin/server` | [server-control.md](../../frontend/docs/pages/server-control.md) | RCON/admin API |
| FD-003 | Mission editor completion | [mission-editor.md](../../frontend/docs/pages/mission-editor.md) | MC Track A/B (DEM, registry) |
| FD-007 | CMS rich text | [content-manager.md](../../frontend/docs/pages/content-manager.md) | WYSIWYG choice |
| FD-008 | Wiki markdown renderer | [wiki.md](../../frontend/docs/pages/wiki.md) | react-markdown |
| FD-009 | Multi-server picker | [server-intel.md](../../frontend/docs/pages/server-intel.md) | UI for `GET /servers` |

Full table: [`frontend/docs/TRACKING.md`](../../frontend/docs/TRACKING.md).

---

## Recently shipped

| Item | Spec | Notes |
|------|------|-------|
| **T-061 drag-move @ 360k (shipped — good enough)** | [t061_drag_move_hotfix.md](../../Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md) | T-061.0 motion ~60 fps + T-061.0.1 `slotIconCache` + slot fast path; mega opts deferred |
| **T-062 incremental bindings (shipped)** | [t062_incremental_bindings.md](../../Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md) | Classifier + bulk delete @ 360k |
| **T-062.2 editor session (shipped)** | [t062_2_editor_session_persistence.md](../../Design_Docs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md) | Alt-tab / warm session fast path |
| **T-060 scale load/save (shipped `b1fd25a`)** | [t060_1](../../Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md) · [t060](../../Design_Docs/Mission_Creator_Architecture/t060_fast_initial_load.md) | Four-phase load; Save @ ~367k/~142 MB → 201 |
| **T-064 Virtualized outliner (shipped)** | [t064_virtualized_outliner.md](../../Design_Docs/Mission_Creator_Architecture/t064_virtualized_outliner.md) | `@tanstack/react-virtual` + segment flatten; scrollable @ ~367k; T-064.1 scroll-ref hotfix |
| **T-063 Spatial index (shipped)** | [t063_spatial_index.md](../../Design_Docs/Mission_Creator_Architecture/t063_spatial_index.md) | rbush pick/marquee @ ~367k |
| **T-059 Bulk paste at scale** | [t059_bulk_paste_operations.md](../../Design_Docs/Mission_Creator_Architecture/t059_bulk_paste_operations.md) | Batch O(n) `pasteSlots`; selection cap 500; outliner virtualization (T-064). **Validated: 360k @ 100+ fps** pan; 6k paste loops smooth |
| **T-058 Toolbelt OBJ/SEL counts** | [t058_entity_count_readout.md](../../Design_Docs/Mission_Creator_Architecture/t058_entity_count_readout.md) | Bottom toolbelt shows OBJ (total placed slots, memoized `selectSlotCount`) + SEL (selected count); mono, right of X/Y/Z; subscribes in the memoized toolbelt so no cursor-move re-render; scale telemetry |
| **T-057 Map perf hotfix** | [t057_map_performance_hotfix.md](../../Design_Docs/Mission_Creator_Architecture/t057_map_performance_hotfix.md) | ≥55 fps pan/zoom @ 200+ slots: cursor → transient store (rAF), drop Deck `onHover` (no per-move pick), pan rAF-coalesce, `React.memo` panels; interactions unchanged |
| **T-056 Ctrl+C/V copy-paste** | [t056_eden_p1_copy_paste.md](../../Design_Docs/Mission_Creator_Architecture/t056_eden_p1_copy_paste.md) | Ctrl/Cmd+C copies slot selection; Ctrl/Cmd+V pastes at cursor (relative layout; off-map +20m nudge); pasted slots selected; one undo step (P1-02) |
| **T-055 Asset browser search** | [t055_asset_browser_search.md](../../Design_Docs/Mission_Creator_Architecture/t055_asset_browser_search.md) | Search field filters the Asset Browser (Factions) tree by name; folder-name match shows subtree; X/Esc clears; filtered leaves still drag (P1-04) |
| **T-054 Attributes entry points** | [t054_attributes_entry_points.md](../../Design_Docs/Mission_Creator_Architecture/t054_attributes_entry_points.md) | Map native dbl-click + ORBAT tree dbl-click → Attributes modal (P1-09); multi-select suppress unchanged |
| **T-053 Ctrl/Cmd additive select** | [t053_eden_p1_additive_select.md](../../Design_Docs/Mission_Creator_Architecture/t053_eden_p1_additive_select.md) | Ctrl/Cmd+LMB toggles slot in/out of selection; Ctrl/Cmd+empty preserves; Shift unbound (P1-01) |
| **T-052 Eden P1 undo keyboard** | [t052_eden_p1_undo_shortcuts.md](../../Design_Docs/Mission_Creator_Architecture/t052_eden_p1_undo_shortcuts.md) | Keyboard undo/redo + `useMissionDoc` StrictMode lifecycle fix; focus-guarded; session edits only |
| **T-050 Cursor Z readout** | [t050_cursor_z_readout.md](../../Design_Docs/Mission_Creator_Architecture/t050_cursor_z_readout.md) | Editor toolbelt CUR mode shows cursor X/Y/**Z** (Z=0 on the flat map until DEM); off-map → dashes; SEL mode unchanged |
| **T-049 Track A quick P0** | [t049_track_a_quick_p0.md](../../Design_Docs/Mission_Creator_Architecture/t049_track_a_quick_p0.md) | Terrain wired to viewport; mission row title/terrain/env hydrate on load; editable X/Y/Z/rotation in Attributes Transform; selection-aware toolbelt readout |
| **T-048 Library create dialog** | [t048_library_create_dialog.md](../../Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md) | `CreateMissionDialog` on `/missions` (header button + My-Missions empty-state CTA + Cmd/Ctrl+N, `mission_maker+`); `/missions/create` route + sidebar item removed; macOS frosted modal |

## Recommended next work

1. **Mission Creator — T-065 cluster/LOD** (MC [ROADMAP](../../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) §Map performance; WIP uncommitted [`t065_cluster_lod.md`](../../Design_Docs/Mission_Creator_Architecture/t065_cluster_lod.md); ~~T-064~~ ✅ [`t064_virtualized_outliner.md`](../../Design_Docs/Mission_Creator_Architecture/t064_virtualized_outliner.md)) → **T-066..T-067** → Eden **T-068+** → **T-070+** terrain base.
2. **Mission editor Track A Phase 2+** — map tiles (A-01), DEM (A-03) — **deferred until Eden P0–P2**
3. **FD-008** — wiki markdown (low risk, high UX)
4. **FD-001** — when backend exposes server/RCON endpoints

---

## Design system

- **Live tokens:** [`frontend/src/index.css`](../../frontend/src/index.css)
- **Reference YAML:** [`Design_Docs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md`](../../Design_Docs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md)
- **Methodology:** [`docs/platform/macos_ux_architecture.md`](../platform/macos_ux_architecture.md)

Do not implement from archived stitch `code.html`.
