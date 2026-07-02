# T-127 — Claude Code handoff (MC UX)

**Slice:** T-127 · **Executor:** claude-code · **Branch:** commit to `main`  
**After:** T-126 @ `4a47688e` (tag **T-126**)  
**Spec:** [`docs/platform/t127_mc_ux_audit_fixes.md`](../../docs/platform/t127_mc_ux_audit_fixes.md)  
**Audit / tracker:** [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) — rows **F2F-03..06**, **F4-01..06** (**ACTIVE**)

**Preflight:** `git pull` · `./scripts/ticket brief T-127`

---

## Execution order

**T-127** is the only active Fable slice — then **T-128** (Cursor docs). Do **not** start T-090.1.2.8 or T-068.

---

## U1 — Conflict server load → IDB persist (MED, trust)

**File:** `apps/website/frontend/src/features/mission-creator/hooks/useMissionEditor.ts` (~521–536)

**Bug:** `resolveConflict('server')` calls blocking `hydrateMissionDoc(md, conflict)` under **`INIT_ORIGIN`**. v2 IDB persist only hooks **`LOCAL_ORIGIN`** updates (`useMissionEditor.ts` ~280–288). Adopted server state never hits IndexedDB → same conflict dialog next cold boot.

**Fix (locked):**
1. Prefer **`hydrateMissionDocWithProgress`** for large payloads (already imported ~217).
2. After hydrate + `applyMissionRowMeta`, **explicitly persist** — do not wait for a user edit:
   - `await saveMissionMetaFromDoc(md, missionId, isDocCancelled)` (`persistence/missionMetaStore.ts`)
   - `await saveSlotsFromDoc(md, missionId, isDocCancelled)` (`persistence/slotChunkStore.ts`)
   - or `await flushMeta(missionId)` + `await flushSlots(missionId)` if debounced queues may hold stale data
3. Keep **`clearEditorSession()`** after server adopt (T-062.2) — already correct.
4. Make `resolveConflict` **async** if needed; wire loading state on conflict dialog buttons in `MissionCreatorPage.tsx` (~328).

**Manual proof:** force conflict → Load server → reload tab → **no repeat prompt**.

---

## U2 — Export compile errors (MED)

**Files:**
- `useMissionEditor.ts` — `exportJson` (~508–519): wrap `compileMission` in try/catch; `toast.error(...)` on failure (pattern: same file ~229, `MissionCreatorPage` ~192).
- Fix return type: `exportJson: () => Promise<void>` on hook interface (~101).
- `TopCommandStrip` / `MissionCreatorPage` — await or `.catch` on export click if still fire-and-forget.

**Manual proof:** trigger compile failure → user sees toast, no unhandled rejection in console.

---

## U3 — Map basemap silent grid (MED)

**Files:**
- `features/tactical-map/state/basemapView.ts` — in `read()`, **coerce** stored `'map'` → `'satellite'` until **T-090.1.1** (comment already says map is disabled).
- Optional one-shot: if key was `'map'`, rewrite localStorage to `'satellite'`.
- `features/tactical-map/layers/useTerrainBasemapLayer.ts` — verify degraded toast still fires when manifest has no pyramid (don't early-return before `onDegraded` when coercing isn't enough).

**Manual proof:** `localStorage.setItem('tbd-mc-basemap-view','map')` → reload editor → satellite or toast, not blank grid-only.

---

## U4 — Layer folder delete confirm (MED)

**File:** `features/mission-creator/layout/LeftOutliner/EditorLayersSection.tsx` (~131–137, delete button ~158)

**Fix:** Before `removeEditorLayer`, if folder has child slots/subfolders (cheap count from store tree), open shared Aegis **`Dialog`** (from `@/components/ui/Dialog`) — not `window.confirm`. Mention subtree size if easy.

**Do not** confirm empty folders if that's noisy — spec says folders **with children**.

---

## U5 — ORBAT 409 messages (LOW, stretch)

**File:** `apps/website/frontend/src/pages/events.tsx` (~395)

Map backend `error` string from mutation failure to distinct toasts (`slot already taken` vs squad reserved) when axios exposes it.

---

## Verify (all exit 0)

```bash
cd apps/website/frontend && npm run build && npm run lint
```

Deliver **`.ai/artifacts/t127_verify_log.md`** — U1–U4 (+ U5 if done) + manual notes.

---

## Return

Commit prefix **T-127:** · tag **T-127** · Co-Authored-By trailer · **Ready for Cursor doc sync** (registry + living tracker).

**Do not edit:** `docs/**`, `.ai/tickets/registry.json`, CLAUDE markers.
