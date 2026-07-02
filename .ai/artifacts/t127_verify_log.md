# T-127 verify log — Fable MC UX audit fixes (U1–U5)

**Branch:** `ticket/T-127` (worktree, base `a6f54ac0`) · **Executor:** claude-code (Fable 5)
**Date:** 2026-07-02 · **Audit rows:** F2F-03..06, F4-01/02/03/05/06

## Gates (all exit 0)

| Gate | Result |
|------|--------|
| `npm run build` (tsc -b && vite build) | ✓ built in ~0.5s (chunk-size warning pre-existing) |
| `npm run lint` (eslint .) | ✓ 0 errors, 0 warnings |
| `npm run test` (vitest) | ✓ 3 files, **26/26** passed |
| `prettier --check` (touched trees) | ✓ clean — see Residuals for pre-existing `useAuthStore.ts` drift |

## U1 — Conflict "Load saved version" → IDB persist (F2F-03 / F4-03)

**Files:** `hooks/useMissionEditor.ts`, `MissionCreatorPage.tsx`

- `resolveConflict` is now async. Server branch: `clearEditorSession()` **before** hydrate (mid-adopt reload cold-boots onto the untouched pre-conflict IDB and safely re-prompts) → `await hydrateMissionDocWithProgress(md, conflict)` (chunked ≥5k slots; was blocking sync `hydrateMissionDoc`) → `applyMissionRowMeta` → explicit persist.
- **Persist path:** `saveMissionMetaFromDocDebounced` + `saveSlotsFromDocDebounced` then `await Promise.all([flushMeta, flushSlots])` — immediate but routed through the per-mission **serialized write chains** (no interleave with a queued autosave; IDB failure surfaces via the existing `notifyAutosaveFailure` toast). Note: the handoff's suggested direct call `saveSlotsFromDoc(md, missionId, isDocCancelled)` would have passed the cancel fn in the `onProgress` slot (real signature `(md, missionId, onProgress?, isCancelled?)`) — the debounce+flush route avoids that and reuses already-imported helpers.
- **Warm-marker re-mark (required for the manual proof):** the ready effect's deps (`docStatus`, `missionId`, `currentSemver`) don't change after a conflict resolve, so nothing re-marked the session after `clearEditorSession()` — an F5 was a **cold** boot, and the cold path prompts on content *presence* (`hasLocalContent`), not divergence: the conflict would re-prompt **even with IDB persisted**. After the flush completes (and `!isDocCancelled()`), `markEditorSessionReady(missionId, { slotCount, currentSemver })` is called explicitly — same-tab reload now takes the T-062.2 warm path (GET skipped, no prompt). This implements what the old inline comment already claimed ("the ready effect re-marks once this state settles").
- Failure path: catch → dev console + `toast.error('Could not apply the server version.')`; `finally` clears busy + closes dialog (mounted-guarded).
- UI: conflict dialog buttons get `disabled={resolvingConflict}` + "Loading saved version…" label; dialog stays open until adopt+persist settle.

**Manual proof (operator, needs running stack):**
1. Open a mission with a saved server version; make a local edit; clear sessionStorage key `tbd-editor-session`; F5 → conflict dialog appears.
2. Click **Load saved version** → buttons disable, then dialog closes on the server state.
3. F5 again → **no repeat prompt** (warm marker + persisted IDB); IndexedDB `tbd-mission-persist` meta/slots records reflect the server payload.

## U2 — Export error/success UX (F2F-04 / F4-05)

**Files:** `hooks/useMissionEditor.ts`, `layout/TopCommandStrip.tsx`

- `exportJson` body wrapped in try/catch: success → `toast.success('Mission JSON exported.')`; failure → dev console + `toast.error`, special-casing worker `could not be cloned` → "non-serializable editor state" (same mapping as `saveVersion`). No more unhandled rejection.
- Interface fixed: `exportJson: () => Promise<void>` (was `() => void` on an async impl); `TopCommandStrip` prop `onExport: () => void | Promise<void>`, button `onClick={() => void onExport()}`.

**Manual proof:** Export → file downloads + success toast. Failure path is code-traced (compile rejection now lands in the catch → toast; verified by type/flow, not forced at runtime).

## U3 — Basemap `'map'` silent grid (F2F-05 / F4-02)

**File:** `tactical-map/state/basemapView.ts` (only — `useTerrainBasemapLayer.ts` unchanged by design)

- `read()` coerces a persisted `'map'` → `'satellite'` with a one-shot localStorage rewrite (inner try/catch for private mode). Module-level `current = read()` runs it once per app load.
- Trace: coerced value un-skips the resolve effect (`if (basemapView !== 'satellite') return` in `useTerrainBasemapLayer.ts:145`) → `resolveSatelliteMode` runs → tiles render, or `mode:'none'` → `onDegraded()` → existing page toast "Satellite basemap unavailable — showing grid only." No silent grid-only state remains.
- Mission Settings "Map" radio is already `disabled` ("ships in T-090.1.1") — no runtime writer of `'map'` exists, so no settings-side toast needed. Remove the coercion when T-090.1.1 ships.

**Manual proof:** `localStorage.setItem('tbd-mc-basemap-view','map')` → reload `/missions/:id/edit` → satellite (or degraded toast) renders, never blank grid; key reads back `satellite`.

## U4 — Folder delete confirm (F4-01)

**Files:** `layout/LeftOutliner/EditorLayersSection.tsx`, `LeftSidebar.tsx`

- Row delete button now calls `requestDeleteFolder`: sole-layer guard first (mirrors `removeEditorLayer`'s ydoc no-op — also stops the old stray selection/active-layer clearing on a delete that would do nothing); subtree walk (same parent-chain approach as ydoc) counts nested subfolders + filed units; **empty folder (0 units, 0 subfolders) deletes immediately** (handoff: confirm only folders with children); otherwise opens the shared Aegis `Dialog` (Base UI — **not** `window.confirm`).
- Dialog copy names the folder and exact counts ("…contains N units and M subfolders. Deleting removes the whole subtree — one undo restores it."), Cancel + destructive `text-error bg-error/20` confirm. Esc/backdrop/× cancel via `onOpenChange`.

**Manual proof:** delete a folder holding units → confirm dialog with counts; confirm → subtree gone, one Ctrl+Z restores; empty folder → immediate delete, no dialog; the only remaining folder → no-op, no dialog.

## U5 (stretch) — ORBAT 409 reasons (F2F-06 / F4-06)

**File:** `pages/events.tsx`

- `apiErrorMessage(e, fallback)` surfaces `response.data.error` (capitalized) when present. Applied to register / reserve / release / assign `onError`s → distinct toasts: "Slot already taken", "Squad is reserved by a leader", "Squad is already reserved", "Registration is closed for this operation" (exact backend strings from `handlers/events.go`), falling back to the old generic lines when no payload.

## Residuals / notes for tracker (no code change here)

1. **F4-03 partial:** a **new-tab** cold boot after a server adopt can still prompt — `onSynced` decides on content *presence*, not local-vs-server divergence. Killing the loop for all cold boots needs divergence tracking (e.g. persisted adopted-semver/hash compared to the GET). Out of T-127's locked scope; warm-path (same-tab reload) loop is fixed.
2. **Pre-existing Prettier drift in `src/store/useAuthStore.ts`** (last touched by T-126 @ `4a47688e`, not in this diff): `npm run format:check` fails on it today on the branch base. Left untouched (T-126/T-128 territory) — flagging because `make ci-local` FMT-2 will trip on main until reformatted.
3. `WORK_HERE.md` (operator worktree note) intentionally left untracked — commit stages explicit paths, not `-A`.
