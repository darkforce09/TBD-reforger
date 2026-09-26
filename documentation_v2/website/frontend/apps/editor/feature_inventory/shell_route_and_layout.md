**Status:** live

# Shell route and layout

The frame the [Mission Creator](/documentation_v2/glossary.md#mission-creator) runs in: the
chromeless route that gives the editor the whole viewport, the chrome laid over the map, the
read-only review workspace that mounts the same page, and the notices the page shows when the
[mission](/documentation_v2/glossary.md#mission) id or the map cannot be used.

## Where it lives

- Code: the route flags in `apps/website/frontend/src/router.rs` and the frame choice in
  `apps/website/frontend/src/v2/pages/navigation/layout.rs`; the page and its chrome in
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` (the
  [app README](/apps/website/frontend/src/v2/apps/editor/README.md)); the chrome dimensions in
  `apps/website/frontend/src/v2/apps/editor/shell/layout.rs` and the review mode in
  `apps/website/frontend/src/v2/apps/editor/shell/review_mode.rs` (the
  [browser session README](/apps/website/frontend/src/v2/apps/editor/shell/README.md)).
- Entry: `/missions/:id/edit`, component `MissionEditorPage`; the
  [route and boot loading](/documentation_v2/website/frontend/apps/editor/feature_inventory/editor_route_loading.md)
  file covers the access tier, the bundle and the boot overlay.
- Related features: [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md)
  (the load-conflict dialog, TBD-CONFLICT-001), [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  (Backspace, E and R), [bottom toolbelt](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md)
  (the debug HUD line).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| SHELL-ROUTER-001 | Chromeless full-viewport editor | shipped |
| SHELL-LAYOUT-001 | Chrome docked over the map | shipped |
| SHELL-UUID-001 | Warning for a mission id that is not a UUID | not built |
| SHELL-CONFLICT-001 | Local-versus-server load conflict dialog | shipped |
| SHELL-FPS-001 | Frame-rate read-out | shipped |
| SHELL-MAPDOWN-001 | "Map unavailable" notice | shipped |
| SHELL-REVIEW-001 | Read-only review workspace | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
SHELL-LAYOUT-001, SHELL-MAPDOWN-001 and SHELL-REVIEW-001 are rows added for shipped code.

### SHELL-ROUTER-001 — Chromeless route

1. The route table marks `/missions/:id/edit` `full_bleed: true`, `chromeless: true` and
   `auth: "mission_maker"` (`apps/website/frontend/src/router.rs:107-111`).
2. `classify_frame` gives every chromeless route the chromeless frame, a
   `h-screen w-screen overflow-hidden` container with no platform sidebar or top bar
   (`apps/website/frontend/src/v2/pages/navigation/layout.rs:62-70`, `:100-104`). Moving between
   the editor and a chromed page swaps the frame; moving between two chromed pages does not.
3. The page reads `:id` from the route; with no parameter it falls back to `draft`
   (`mission_editor.rs:195-202`).

### SHELL-LAYOUT-001 — Chrome over the map

1. The map canvas fills the page; one chrome layer sits above it and stops pointer presses from
   reaching the map (`mission_editor.rs:255-265`).
2. The layer holds the top command strip in a 48 px band, the left and right docks, 240 px wide
   and 24 px when collapsed, the floating mode toolbar, the status bar and the edge grid
   references (`mission_editor.rs:266-338`; the numbers in `shell/layout.rs:22-60`).
3. The dialogs and overlays (Attributes, Mission Settings, the faction and ORBAT managers, the
   conflict dialog, the tab-lock banner, the context menu, the comment editor, the Connections
   panel, the asset picker, the tool overlays, the transform widget and the snap read-out) mount
   in the same layer (`mission_editor.rs:339-381`).
4. Backspace hides or shows all the chrome; E and R collapse the left and right docks
   (KEY-CHROME-001). The live insets that tell the map from the chrome follow these flags
   (`shell/layout.rs:170-210`).

### SHELL-UUID-001 — Invalid mission id

Not built. No banner warns about an id that is not a UUID. The boot skips the server fetch for
such an id and keeps the seeded document as a local draft
(`shell/hydrate/server_reconciliation.rs:124`, `is_uuid` in
`apps/website/map-engine/src/editing/persist/mission_id.rs:16`); Save Version still posts to
`/missions/{id}/versions`, and the server's refusal shows in the save status.

### SHELL-CONFLICT-001 — Load conflict dialog

The same feature as TBD-CONFLICT-001: the "Unsaved local changes" dialog is described with the
boot restore in [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md#data-hyd-001-and-tbd-conflict-001--boot-restore-and-reconciliation),
and its keys in KEY-DIALOG-001.

### SHELL-FPS-001 — Frame rate

1. Once a second the frame loop writes a debug line such as
   `z -2.00 · c12 · glyph 340 · 58 FPS · rf 1.20ms (833 eq)`: the zoom, the resident chunks, the
   tree glyphs, the frames drawn in the last second and the render CPU time
   (`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs:40-58`).
2. The line sits in the status bar and is hidden until Ctrl/Cmd+Alt+D shows it
   (BOTTOM-DEBUG-001). The loop draws only when something changed, so an idle map reads a low
   figure.

### SHELL-MAPDOWN-001 — Map unavailable

When the render engine fails to start, the boot overlay shows the failed segment
(`mission_editor/canvas_mount/boot_tasks.rs:355-367`). After "Continue without map", a pill
above the toolbelt reads "Map unavailable" with the reason, and editing goes on without a map
(`mission_editor.rs:448-462`).

### SHELL-REVIEW-001 — Review workspace

1. `/missions/:id/artifacts/:artifact_id/workspace` opens review mode on the version an
   [artifact](/documentation_v2/glossary.md#artifact) compiled from, then mounts the same
   `MissionEditorPage` (see the [review workspace page](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md)).
2. While review mode is open, the boot restores the reviewed version, no draft is written, the
   row mirrors patch nothing, and Save Version answers "The review workspace of artifact …,
   version …, saves nothing — open the mission in the Mission Creator to change it."
   (`shell/review_mode.rs:109-117`).

## Data

- No call of its own. The route's server and draft restore is in
  [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md);
  the review workspace reads `GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace`
  through its page.

## Design

- The editor owns the viewport: no platform chrome, the map under every panel, and the panels
  as frosted layers over it.
- Design target: the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md)
  and Eden's workspace in the [Eden UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md).
  Differences: the tools sit in a floating pill above the status bar, and the frame-rate figure
  is a debug line rather than an always-on counter.

## Open work

- [T-158 — Editor shell UX consolidation](/documentation_v2/tickets/specs/t158_editor_shell.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-158_plan.md)): one settings entry point and
  every inert top-bar button wired.
- [T-142 — MC shell layout polish](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-142_plan.md)): toolbelt placement and
  Attributes grouping.
- [T-927 — Editor chrome dblclick leak to map](/documentation_v2/tickets/specs/t927_chrome_dblclick_leak.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-927_plan.md)): a double-click on the docks, the
  Attributes dialog or the top bar stays off the map.
- [T-719 — Debug HUD: invisible under DockRight; AltGr chords spuriously toggle it](/.ai/tickets/T-719.toml)
  (deferred, no plan): the debug line, frame rate included, stays visible.

No open ticket covers a warning for a mission id that is not a UUID.

## Decisions

- The route table declares the chromeless layout once, and the frame reads it: adding a
  full-viewport route is a table edit, never a change to the frame.
- The frame rate is a debug read-out, off by default: the status bar keeps its room for the
  read-outs a mission maker uses.
