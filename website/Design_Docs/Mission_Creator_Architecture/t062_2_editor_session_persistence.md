# T-062.2 — Editor session / background-tab resilience

**Status:** **shipped** — manual verify @ ~360k (Firefox dev): alt-tab extended period → no automatic load overlay; edits preserved  
**Git tag on ship:** **T-062.2** (`693e227`)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t062_incremental_bindings.md](t062_incremental_bindings.md)

**Prerequisites:** T-062 shipped (`a5a651d`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~360k slots).

---

## Problem

After alt-tabbing away from `/missions/:id/edit` for an extended period (Firefox dev), the **loading overlay reappeared on its own** — "Reading local save…" / "Downloading mission…" — without F5 and with the URL unchanged. At ~360k this meant a multi-minute cold boot (IDB replay + multi-MB server GET + possible spurious conflict dialog).

**Root cause (dev):** Vite HMR WebSocket disconnect while the tab is backgrounded; on reconnect `@vite/client` triggers a **full page reload**, cold-booting `useMissionDoc`.

**Amplifier:** Every **cold** boot always ran `GET /missions/:id` (full `json_payload`) even when local persistence already held the working draft → conflict prompt when `hasLocalContent(md)`. **T-062.2** warm path skips GET when session marker + local content.

**Secondary:** `yieldToUi()` and the restore rAF poll blocked on `requestAnimationFrame`, which is suspended in background tabs — in-progress loads froze until refocus.

---

## Shipped implementation

### Part 1 — Dev: block Vite full reload on editor route

| File | Change |
|------|--------|
| **NEW** [`frontend/src/dev/viteReloadGuard.ts`](../../frontend/src/dev/viteReloadGuard.ts) | `vite:beforeFullReload` → reassign `payload.path` to block reload on `/missions/:id/edit` (Vite #5763 — throw does not work). Dev `pageshow` + navigation-type diagnostics. |
| [`frontend/src/main.tsx`](../../frontend/src/main.tsx) | `if (import.meta.env.DEV) import('@/dev/viteReloadGuard')` |

**Note:** WS reconnect may call `location.reload()` directly (bypassing `beforeFullReload`). No vite.config plugin was needed after manual verify on Firefox dev — primary mechanism sufficient.

### Part 2 — Warm session fast path (dev + prod)

| File | Change |
|------|--------|
| **NEW** [`frontend/src/features/mission-creator/hooks/editorSession.ts`](../../frontend/src/features/mission-creator/hooks/editorSession.ts) | `sessionStorage` key `tbd-editor-session` → `{ missionId, readyAt, slotCount, currentSemver }`; 24h TTL; per-tab scope |
| [`useMissionEditor.ts`](../../frontend/src/features/mission-creator/hooks/useMissionEditor.ts) | On `docStatus === 'ready'` → `markEditorSessionReady`. In `onSynced`: if warm + `hasLocalContent(md)` → skip GET, restore semver. Clear on cold load (`!hasLocalContent`), `resolveConflict('server')`. Refresh marker on `saveVersion` success. |
| [`useMissionDoc.ts`](../../frontend/src/features/mission-creator/hooks/useMissionDoc.ts) | Dev mount/unmount debug logs |

**Tradeoff:** Warm path trusts local IndexedDB. Remote server changes since last ready are **not** detected until a cold load (new tab, expired TTL, cleared session, or `resolveConflict('server')`).

### Part 3 — Background-safe progress

| File | Change |
|------|--------|
| [`yieldToUi.ts`](../../frontend/src/features/tactical-map/state/yieldToUi.ts) | When `document.hidden`: `setTimeout(0)` only (skip rAF) |
| [`useMissionDoc.ts`](../../frontend/src/features/mission-creator/hooks/useMissionDoc.ts) | Restore poll: rAF when visible, `setInterval(500ms)` when hidden; switch on `visibilitychange` |

---

## Load paths after T-062.2

| Scenario | Overlay phases | Server GET |
|----------|----------------|------------|
| **Cold** (first visit, new tab, empty IDB) | restoring → download → apply → local | Yes |
| **Cold conflict** (IDB + server both have content, no warm marker) | Same + conflict dialog | Yes |
| **Warm return** (same tab, warm marker + IDB content) | restoring → local flush only | **Skipped** |
| **Dev alt-tab** (Part 1 blocks reload) | **None** — live Y.Doc survives | N/A |

---

## Acceptance (T-062.2 — shipped)

| Check @ ~360k (Firefox dev) | Result |
|-----------------------------|--------|
| Alt-tab 30+ min → tab back | **Pass** — no automatic overlay |
| Edits before alt-tab preserved | **Pass** |
| Cold first visit (new tab) | **Pass** — still GETs + hydrates |
| Save Version → 201 | **Pass** (regression) |
| build + lint | **Clean** |

---

## Out of scope

- Ref-counted module-level `MissionDocSession` (Fix D)
- ~~T-062.1.1 Save orbat dedup~~ ✅ — spec [`t062_1_1_batch_save.md`](t062_1_1_batch_save.md)
- Backend lightweight metadata endpoint
- Prod tab-discard prevention (memory / DeckGL)

---

## After T-062.2

- ~~**T-062.1** chunked IDB load~~ ✅ — spec [`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md)
- ~~**T-062.1.1** Save orbat dedup~~ ✅
- ~~**T-063** spatial index~~ ✅ — spec [`t063_spatial_index.md`](t063_spatial_index.md)
- **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**
