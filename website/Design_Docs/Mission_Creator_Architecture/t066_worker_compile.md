# T-066 — Worker offload: compile + version blob assembly

**Status:** **Shipped** — T-066 + **T-066.1** hotfix; FE build/lint clean; manual @ ~367k: Save Version **201** (user verified 2026-06-25).  
**Git tag:** **T-066** (this commit)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [engineering_plan.md](engineering_plan.md) §Phase 9 · [t060_1_scale_load_save_completion.md](t060_1_scale_load_save_completion.md) · [t065_cluster_lod.md](t065_cluster_lod.md)

**Prerequisites:** T-065 shipped (`845bfb2`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~367k slots).

---

## In one sentence

**Move `compileMissionWithProgress` and `buildVersionBlob` off the main thread so Save Version / Export stay responsive @ 367k and unblock the ≤10 s @ 1M stretch goal.**

---

## Problem (pre-T-066)

T-060/T-060.1 ran compile + blob assembly on the **main thread** with chunked yields ([`compile.ts`](../../frontend/src/features/mission-creator/compiler/compile.ts)). Save @ ~367k worked (~142 MB → 201) but compile/prepare competed with Deck/React. Stretch: ≤10 s @ **1M** needs worker offload ([`engineering_plan.md`](engineering_plan.md) Phase 9).

---

## Goal

- Save + Export compile in **`compiler.worker.ts`** via Comlink.
- Progress UX and payload contract unchanged (T-062.1.1 `omitOrbat` on Save; T-060.1.2 Blob POST).
- **Never pass raw `useMapStore.getState()`** to the worker — see T-066.1.

**Out of scope:** registry worker, server Enfusion compile (T-072), upload in worker, T-067 chunks, `docToSnapshot` on hydrate.

---

## Implementation (working tree — T-066)

| File | Role |
|------|------|
| `compiler/compiler.worker.ts` | Comlink.expose `{ compileMission, compileMissionWithProgress, buildVersionBlob }` |
| `compiler/compilerClient.ts` | Singleton worker + `Comlink.wrap`; `Comlink.proxy(onProgress)` |
| `compiler/compile.ts` | Worker-safe leaf imports (`coords/terrains`, type-only `MapSnapshot`); `setTimeout(0)` yield in `buildVersionBlob` (no `yieldToUi`) |
| `hooks/useMissionEditor.ts` | Save/Export via `compilerClient`; `terminateCompiler()` on unmount |
| `state/useMapStore.ts` | **`pickMapSnapshot`** (T-066.1) — strips Zustand actions before worker RPC |
| `tactical-map/index.ts` | Re-export `pickMapSnapshot` |

**Worker bundle:** ~6.9 KB ESM chunk (`compiler.worker-*.js`) — comlink + compile + terrains only; no Deck/React.

---

## T-066.1 hotfix — `pickMapSnapshot` (required)

**Bug:** Initial T-066 passed `useMapStore.getState()` (`MapStoreState`) to Comlink. Store includes action functions (`setSelection`, `_applySnapshot`, …). `structuredClone` → **DataCloneError 25** ("Function object could not be cloned") at compiling phase (~2s after walking 367k slots). Misreported as "Could not reach the server (25: …)" in Save dialog.

**Fix:** `pickMapSnapshot(state)` returns only the 10 `MapSnapshot` entity-dictionary fields. **Always** call before `compileMission*` / worker RPC:

```ts
const snapshot = pickMapSnapshot(useMapStore.getState())
```

**Locked rule:** `getState()` is **never** worker-safe. `MapSnapshot` plain data only.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| RPC | Comlink |
| Worker lifetime | Singleton per editor session; `terminateCompiler()` on mission unmount |
| Snapshot transfer | **`pickMapSnapshot(getState())`** then structuredClone in Comlink |
| Small missions | Always worker (one path) |
| Progress | `Comlink.proxy(onProgress)` |
| Upload / size gates | Main thread unchanged after blob returns |

---

## Acceptance (ship gate)

| Check | Bar | Status |
|-------|-----|--------|
| Save @ ~367k | **201** | ✅ user verified |
| Export JSON | Valid `orbat[]` + `editor` | Pending formal sign-off |
| `npm run build` + `lint` | Clean | ✅ |
| `pickMapSnapshot` before worker | No DataCloneError | ✅ |
| Worker teardown | No leak on route leave | Pending optional check |
| Git tag T-066 | Committed (`53bc2a8`) | ✅ |

**Stretch:** ≤10 s compile+prepare @ 1M — profile after ship; may need T-066.1 tuning or T-067.

---

## After T-066 ship

- **T-067** ✅ shipped — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Next:** Eden **T-068+**
- Eden **T-068+**

---

## Documentation sync (T-066 shipped)

Shipped in T-066 commit: `CLAUDE.md` §Status, `agent_execution.md`, `ROADMAP.md`, `feature_inventory.md` (PERF-WORKER-001), `TAGS.md`, `docs/AGENT_COMMIT_CHECKLIST.md`, spec footers.

---

## Claude Code prompt archive

Historical — T-066 + T-066.1 implemented in working tree. Do not re-run unless regressing Save/Export worker path.
