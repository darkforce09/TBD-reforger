**Status:** live

# Data persistence and compile

Where a [mission](/documentation_v2/glossary.md#mission) lives while the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) edits it: the local draft in the
browser, the restore and server reconciliation at boot, the compile to the saved payload, and what
stands in for collaboration.

## Where it lives

- Code: the draft store in [`apps/website/frontend/src/v2/apps/editor/shell/persist/`](/apps/website/frontend/src/v2/apps/editor/shell/persist/README.md)
  and `apps/website/frontend/src/v2/apps/editor/shell/persist.rs`; the boot restore and server
  reconciliation in [`apps/website/frontend/src/v2/apps/editor/shell/hydrate/`](/apps/website/frontend/src/v2/apps/editor/shell/hydrate/README.md);
  the cross-tab writer lock in [`apps/website/frontend/src/v2/apps/editor/shell/tab_lock/`](/apps/website/frontend/src/v2/apps/editor/shell/tab_lock/README.md);
  Save Version and the exports in [`apps/website/frontend/src/v2/apps/editor/shell/document_commands/`](/apps/website/frontend/src/v2/apps/editor/shell/document_commands/README.md);
  the document handle and seed in [`apps/website/frontend/src/v2/apps/editor/bridge/document_host/`](/apps/website/frontend/src/v2/apps/editor/bridge/document_host/README.md);
  the record keys, the local-versus-server classification and the adoption in
  [`apps/website/map-engine/src/editing/persist/`](/apps/website/map-engine/src/editing/persist/README.md);
  the CRDT document in [`apps/website/map-engine/src/data/store/`](/apps/website/map-engine/src/data/store/README.md);
  the payload compiler in [`apps/website/map-engine/src/data/scenario/compiler/`](/apps/website/map-engine/src/data/scenario/compiler/README.md).
- Entry: the canvas mount's boot tasks
  (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/boot_tasks.rs`) restore
  the draft and hydrate from the server; the undo driver schedules every draft write.
- Related features: [editor route and boot loading](/documentation_v2/website/frontend/apps/editor/feature_inventory/editor_route_loading.md),
  [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  (the load-conflict dialog). Save Version and Export JSON are buttons of the top command strip's
  inventory; this file covers what they send.

## Behaviour

| ID | Feature | Status |
|---|---|---|
| DATA-IDB-001 | Local draft in IndexedDB | shipped |
| DATA-SEED-001 | Seed of a new document | shipped |
| DATA-HYD-001 | Server hydrate at boot | shipped |
| DATA-HYD-ORBAT-001 | Rebuild a document from an ORBAT-only payload | not built |
| DATA-COMP-001 | Compile to the saved payload | shipped |
| DATA-COLLAB-001 | Realtime multi-user editing | not built |
| DATA-TABLOCK-001 | One writer tab per mission in a browser | shipped |
| DATA-HYD-TITLE-001 | Title from the server's mission row | shipped |
| DATA-CONFLICT-EDGE-001 | What counts as a local change worth a conflict | partial |
| TBD-CONFLICT-001 | Local-versus-server conflict dialog (not in Eden) | shipped |
| DATA-SIGNOUT-001 | Drafts dropped on sign-out | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
DATA-HYD-ORBAT-001 splits the ORBAT fallback out of DATA-HYD-001; DATA-TABLOCK-001 and
DATA-SIGNOUT-001 are rows added for shipped code.

### DATA-IDB-001 — Local draft

1. The draft is one whole-document CRDT update blob in the IndexedDB database `tbd-mission-yrs`,
   store `doc-state`, keyed per account and mission (the owner is the signed-in account, or
   `anon`).
2. After every committed edit the undo driver schedules a write once the boot restore has
   settled; a 1000 ms idle debounce folds a burst of edits into one write, and the write flushes
   at once when the tab is hidden or the page is left. Review mode writes nothing.
3. A write is dropped when the account changed, deferred in a read-only tab, refused when the
   document holds nothing authored or when an existing record could not be read, and merged
   first (a CRDT union) when the stored record moved on.
4. The top strip reads "Draft saved just now" (then seconds, minutes, hours ago), with the
   tooltip "Your work is auto-saved as a local draft in this browser, so closing the tab will not
   cost you the session — use Save Version to publish a durable version to the mission library."
   Failures show "Save failed: {reason}", "Retrying local backup ({n}/3)", "Local backup
   unreadable" with "Retry", or, on a full disk, "quota: browser storage is full — the draft was
   not saved".
5. The draft never reaches the server: Save Version is the only version write. Time, weather and
   the Mission Settings row fields are the exception; they patch the mission row, debounced.
6. Leaving the page with unsaved changes on a real mission asks "You have unsaved mission
   changes."

### DATA-SEED-001 — Seed

1. Every mount starts from a seeded document, eight fixture slots and template comments, written
   outside the undo history.
2. On a real mission with an empty saved payload and no draft, the hydrate replaces the seed with
   the empty payload and the mission row, and adds the default folder `layer-1`, "Default Layer".
   A mission id that is not a UUID has no server step, so its seed stays and is saved as a draft.

### DATA-HYD-001 and TBD-CONFLICT-001 — Boot restore and reconciliation

1. The boot reads the IndexedDB draft into a fresh document (a restored draft counts as
   unsaved), then always fetches `GET /api/v1/missions/{id}` for a UUID id.
2. A 404 keeps the local copy silently; any other failure keeps it with the toast "Could not load
   the saved version — editing your local copy."
3. No draft, or a draft with nothing authored: the server version is adopted, clean. A draft that
   compiles to the same authored content: kept, marked clean. A draft that differs: the "Unsaved
   local changes" dialog.
4. The dialog compares "Your local copy" and "Server version" (object counts, when each was
   written) and warns "Loading the server version will discard your local copy. One Ctrl/Cmd+Z
   puts it back." "Keep local copy" keeps the draft, unsaved. "Load server version" snapshots the
   draft first, adopts the server version as one undo step and toasts "Loaded the server version.
   Your local copy ({n} objects) was backed up — press Ctrl/Cmd+Z to put it back."

### DATA-HYD-ORBAT-001 — ORBAT-only payloads

Not built: the hydrate reads the payload's `editor` block only and skips `orbat`, so a payload
that carries an ORBAT without an `editor` block loads no slots.

### DATA-COMP-001 — Compile

1. `compile_payload` in `apps/website/map-engine/src/data/scenario/compiler/payload/` turns the
   document into the payload `contracts_v2/definitions/mission-editor-payload.schema.json`
   describes: `schemaVersion`, `map`, `environment`, `loadouts`, `objectives`, `vehicles`,
   `entities`, `markers`, the `editor` block (`factions`, `squads`, `slots`, `editorLayers`), the
   title when set, and the zones, triggers, comments, connections and compositions.
2. It runs on the main thread, on Save Version (without the ORBAT, which the server derives) and
   Export JSON (with it, in an envelope with `exportFormatVersion: 1`); Export Compiled flattens
   the document to `contracts_v2/definitions/mission.schema.json`. The boot's conflict check
   compiles both sides too.
3. Save Version sends `{semver, editor_notes, payload}`; the client refuses duplicate slot ids,
   and the server validates the schema, wire safety, cargo and SemVer and refuses an empty
   payload.

### DATA-COLLAB-001 and DATA-TABLOCK-001 — Collaboration and tabs

1. No realtime collaboration exists: two people saving the same mission each create a version,
   and the last one saved is current. A version number already taken is refused with 409.
2. Within one browser, the tabs holding a mission elect one writer (a Web Lock, else the oldest
   tab, coordinated over a BroadcastChannel); only it writes the draft, and the others pull the
   record when it announces a save. A reading tab shows "Read-only tab" and says another tab has
   the mission open.

### DATA-HYD-TITLE-001 — Title

Adopting a server version takes the payload's title, else the mission row's; a warm open with an
empty saved payload stamps the row's title. Saving copies a non-blank payload title onto the
mission row.

### DATA-CONFLICT-EDGE-001 — Conflict scope

A draft counts as authored when it holds any faction, slot, objective, vehicle, world object,
zone, composition, comment or marker. It differs from the server only when the slot counts differ
or the compiled `editor`, `loadouts`, `objectives`, `vehicles` or `markers` differ.

### DATA-SIGNOUT-001 — Sign-out

Signing out deletes the departing account's drafts and snapshots; a signed-in boot evicts drafts
that belong to other accounts.

### Known discrepancies

- The Save Version dialog pre-fills "0.1.0" (`apps/website/frontend/src/v2/apps/editor/mission_editor.rs`)
  and never changes it — creating a mission already inserts version `0.1.0`
  (`create_mission` in `apps/website/api_v2/src/missions/handlers/mission_lifecycle.rs`), so the
  default first save of a new mission is refused with "Version 0.1.0 already exists".
- A draft that differs from the server only in zones, triggers, comments, connections,
  compositions, world objects, the title or the environment is classified as matching and marked
  clean (`apps/website/map-engine/src/editing/persist/local_versus_server.rs`,
  `apps/website/frontend/src/v2/apps/editor/shell/hydrate/server_reconciliation.rs`), so the
  unsaved dot and the unload prompt disappear while that work exists only in the browser.
- "Load server version" adopts with an empty mission row, so a payload without a title leaves the
  mission untitled (`server_reconciliation.rs`,
  `apps/website/map-engine/src/data/store/rows/hydrate.rs`).
- A re-hydrate keeps connections the new payload lacks (`hydrate.rs`).
- The conflict dialog's object counts count slots only, and it mixes a relative local time with
  an absolute server time.

## Data

- `GET /api/v1/missions/{id}` (`get_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_library.rs`): the mission row and its current
  version's payload, read once at boot for a UUID id.
- `POST /api/v1/missions/{id}/versions` (`create_version` in
  `apps/website/api_v2/src/missions/handlers/mission_versions.rs`): creates an immutable version
  with a new SemVer; the route alone lifts the body limit to the configured version size
  (256 MiB by default); a taken SemVer is refused with 409; a non-blank payload title becomes the
  mission's title.
- `PATCH /api/v1/missions/{id}` (`update_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_lifecycle.rs`): time and weather (400 ms
  debounce) and the Mission Settings row fields such as the game mode, mirrored onto the mission
  row.
- IndexedDB `tbd-mission-yrs` / `doc-state` (drafts and backup snapshots), localStorage
  `tbd-mission-draft:<key>` (write stamps), sessionStorage `tbd-editor-session` (the warm-session
  marker, per account, 24 hours).

## Design

- The draft is invisible until it matters: the top strip's "Draft saved" chip, the save-status
  line and the conflict dialog are its only surfaces.
- Design target: the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md)
  and the [decisions log](/documentation_v2/website/frontend/apps/editor/decisions.md).

## Open work

- [T-821 — Save version prefill static; second save 409s](/documentation_v2/tickets/specs/t821_save_version_prefill.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-821_plan.md)): the dialog proposes a free
  version number after each save.
- [T-295 — Realtime collaborative editing](/documentation_v2/tickets/specs/t295_realtime_collab.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-295_plan.md)) and
  [T-132 — Multiplayer MC + visual git](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-132_plan.md)): several mission makers edit one
  mission at once.
- [T-937 — Editor data layer: id arrays, undo, persist](/documentation_v2/tickets/specs/t937_editor_data_layer.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-937_plan.md)): the document's id arrays,
  undo and persistence are reworked.
- [T-932 — Parked briefing markers survive server save/reload](/documentation_v2/tickets/specs/t932_parked_markers_persist.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-932_plan.md)): parked markers reach the saved
  payload.
- [T-140 — Mission client payload budget](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-140_plan.md)): a size budget for the payload
  the client loads.
- [T-1050 — Fix mission re-hydrate keeping stale connections](/.ai/tickets/T-1050.toml)
  (idea, no plan): a re-hydrate clears connections first.
- [T-946.79 — Conflict modal mixes relative and absolute times](/.ai/tickets/T-946.79.toml)
  (idea, no plan): both sides show one time format.
- [T-093 — Continuous autosave polish](/.ai/tickets/T-093.toml),
  [T-840 — Draft chip saved just now on boot before edit](/.ai/tickets/T-840.toml) and
  [T-717 — Continue-without-map before hydrate resurrects the boot overlay forever](/.ai/tickets/T-717.toml)
  (deferred, no plan): draft-chip and boot polish.

## Decisions

- Autosave writes only the local draft; the server gets a version only when the mission maker
  saves one, so every server version is a deliberate, immutable SemVer.
- A warm return always fetches the server: a draft is never trusted to be the latest version.
- Loading the server version over a draft is itself one undo step, with the draft snapshotted
  first: the choice in the conflict dialog is reversible.
- Versions are immutable and numbered by the mission maker: a taken number is refused rather than
  overwritten.
