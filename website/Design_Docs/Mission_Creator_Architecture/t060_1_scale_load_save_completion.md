# T-060.1 — Scale load/save completion (determinate progress + 360k acceptance)

**Status:** **T-060 + T-060.1 + T-060.1.1 + T-060.1.2 + T-060.1.3 + T-060.1.4 shipped** — load partial pass @ ~360k; **Save @ ~367k / ~142 MB → 201** (browser + curl 140 MB verified).
**Git tag on ship:** **T-060** (commit `b1fd25a`, 2026-06-23 — T-060 + T-060.1 + T-060.1.1 + T-060.1.2 + T-060.1.3 + T-060.1.4)
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE
**Builds on:** [t060_fast_initial_load.md](t060_fast_initial_load.md) (T-060 code landed; acceptance slices T-060.1 → **T-060.1.3**)

**Prerequisites:** T-057–T-059 shipped; T-060 **shipped** in `b1fd25a` (256 MB route, bulk sync, chunked compile, overlay + T-060.1..T-060.1.4 acceptance).

---

## Why T-060.1 exists

Manual verify @ **~300k objects** (2026-06):

| Issue | Observed | T-060 claimed | Gap |
|-------|----------|---------------|-----|
| **Load time** | **~30 s–1 min** (T-060.1.1 partial pass) | Overlay + one coalesced flush | **T-062.1** v2 chunked restore ✅ |
| **Load bar** | Indeterminate (T-060) → stuck 0% (T-060.1) → **restoring label, 0→300k jump** (T-060.1.1 legacy v1) | Real % throughout | **T-062.1** v2 determinate `done/total` ✅ |
| **Save** | ~~Upload ~4% → ERR_NETWORK~~ | 360k → 201 | **FIXED T-060.1.4** (stale API + 1 MB global wrap; curl 140 MB → 201; browser verify pending) |
| **Hydrate bulk window** | — | “hydrate in bulk coalesce” | `endBulkSync()` runs **before** server hydrate completes |

T-060 was the **foundation** (gate, coalesce, API cap, compile progress). **T-060.1 completes acceptance.** **T-061** drag-move shipped (good enough). **T-062** incremental bindings @ 360k. **T-062.2** editor session / alt-tab. **T-062.1** chunked IDB load. **T-062.1.1** Save orbat dedup. **T-063** spatial index. **T-064** virtualized outliner. **T-065** cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ spatial chunks — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**.

**North-star reminder:** Linear load time @ 300k → ~10 min @ 10M without incremental IDB + bindings at scale. T-060.1 targets **360k acceptance** (determinate UX + save works); **≤10 s @ 1M** remains **T-066** stretch. **T-062.1** shipped v2 chunked IDB restore.

---

## Acceptance (T-060.1 — required before T-060 commit)

### Load

- **Determinate** loading overlay: **0–100%** with phase label and entity count where applicable.
- Phases (weighted progress — **execution order**, see locked decisions):
  1. **Restoring** — IndexedDB replay into Y.Doc (`restoring` phase; poll slot count via rAF) — **T-060.1.1**
  2. **Downloading** — `GET /missions/:id` with `onDownloadProgress` (when applicable)
  3. **Applying** — chunked `hydrateMissionDocWithProgress` (when server payload non-empty)
  4. **Local flush** — coalesced `docToSnapshotWithProgress` + `_applySnapshot` at `endBulkSync`
- **Bug fix:** server hydrate runs **inside** `beginBulkSync`/`endBulkSync`; `endBulkSync()` only after reconcile + hydrate settle.
- **360k mission:** load completes with visible %; record wall time in §Shipped (stretch ≤60 s @ 360k; ideal path to ≤10 s @ 1M is T-062+).

### Save

- **360k Save Version → 201** (manual, dev stack `make api && make web`).
- Upload phase reaches server (not `!resp` network error).
- Error messages include axios `code`/`message` when no response (e.g. `ECONNABORTED`, `ERR_NETWORK`).
- Compiling progress unchanged (worked in manual test).

### Engineering

- `npm run build && npm run lint` clean.
- `make test-it` clean.
- Small mission (<500 slots): no regression.

---

## Root causes (confirmed in code review)

### Load — IndexedDB replay dead zone (T-060.1 manual verify)

```typescript
// useMissionDoc.ts effect start → bindStoreToDoc → new IndexeddbPersistence(...)
// y-indexeddb replays 300k slots into Y.Doc synchronously on the main thread
// NO reportLoad() calls until persistence.once('synced') fires
// Overlay sits at INITIAL_LOAD { value: 0 } for minutes — user sees stuck 0%
// Firefox: "This page is slowing down Firefox"
// Alt-tab "fix" is coincidence: replay completes while away; user returns to later phases
```

T-060.1 progress (download/apply/local) only starts **after** `synced`. For returning sessions with a large local cache, IDB replay is often the longest phase.

### Load — one synchronous snapshot

```typescript
// bindings.ts endBulkSync → flush → docToSnapshot(md)
// e.slots.toJSON() × 300k = multi-minute main-thread block
```

Bulk sync reduced **N flushes → 1 flush** but not **O(n) snapshot cost**.

### Load — hydrate outside bulk window

```typescript
// useMissionDoc.ts (current)
const reconcile = onSyncedRef.current?.(md)
endBulkSync()  // ← too early
Promise.resolve(reconcile).finally(() => setDocStatus('ready'))
// hydrateMissionDoc runs after endBulkSync → extra full flush
```

### Save — no HTTP response

```typescript
// useMissionEditor.ts
if (!resp) return { ok: false, error: 'Could not reach the server...' }
```

Likely contributors @ 300k compiled payload:

1. Axios default — no extended timeout on huge POST; dev proxy may drop long uploads.
2. Synchronous `JSON.stringify` of ~100MB+ body before upload starts (`assemblePayload` also sync after compile % hits 100%).
3. Vite proxy `/api` — no explicit `timeout` / large-body tuning.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| **Tag** | Same **T-060** git tag on ship (T-060.1 = acceptance completion pass) |
| **Load progress model** | Weighted phases (**execution order**): **restoring 0–15%** (T-060.1.1), **download 15–35%**, **apply 35–55%**, **local flush 55–100%**. Skip download/apply bands when server path not taken (empty payload / conflict-only GET). |
| **Determinate bar** | Show `%` + phase label + `"N / M objects"` when slot total known |
| **Chunk size** | ~5000 entities per yield (match compile chunk) |
| **docToSnapshot** | New `docToSnapshotWithProgress(md, onProgress)` — yield every chunk while building `slotsById` (largest map) |
| **hydrateMissionDoc** | New `hydrateMissionDocWithProgress(md, payload, onProgress)` — yield every chunk in slot loop |
| **Bulk sync timing** | `beginBulkSync` before binding; **keep open through server hydrate**; single `endBulkSync` after hydrate |
| **Save timeout** | Version POST: `timeout: 600_000` (10 min), `maxBodyLength: Infinity`, `maxContentLength: Infinity` |
| **Vite proxy** | `server.proxy['/api'].timeout = 600_000` |
| **Save body serialization** | Chunked stringify or `transformRequest` that yields — minimum: extend timeout + surface real error; stretch: gzip request body (optional T-060.1 stretch) |
| **assemblePayload** | Yield during `editor.slots` / large map copies (compile hits 100% before assembly finishes today) |
| **≤10 s @ 1M** | Out of scope for T-060.1 — document dependency on **T-062.1** ✅ (v2 IDB) + **T-066** |

---

## Implementation specification

### B1 — Fix bulk-sync timing (`useMissionDoc.ts`)

```typescript
persistence.once('synced', () => {
  seedMeta(...)
  seedDefaultLayer(...)
  const reconcile = onSyncedRef.current?.(md)
  Promise.resolve(reconcile).finally(() => {
    endBulkSync()  // ONE flush: IDB + seed + server hydrate
    if (mountedHere) setDocStatus('ready')
  })
})
```

`onSynced` must wrap server hydrate in bulk (already open from effect start). Do **not** call `endBulkSync` before reconcile settles.

### B2 — Determinate load progress (`loadProgress` state)

Add to `MissionDocHandle` / `MissionEditorHandle`:

```typescript
export type LoadPhase = 'local' | 'downloading' | 'applying'
export interface LoadProgress {
  phase: LoadPhase
  value: number  // 0..1 overall
  label: string  // e.g. "Applying server data…"
  done?: number
  total?: number
}
```

Thread `onLoadProgress` callback from `useMissionDoc` → overlay.

### B3 — `docToSnapshotWithProgress` (`bindings.ts`)

- Async variant; yield every `chunkSize` slots when building `slotsById`.
- `endBulkSync` flush calls `await docToSnapshotWithProgress(...)` instead of sync `docToSnapshot`.
- Report progress into **local** phase (0–0.5 overall).

### B4 — Download progress (`useMissionEditor.ts` `onSynced`)

```typescript
api.get(`/missions/${missionId}`, {
  onDownloadProgress: (e) => onLoadProgress?.({ phase: 'downloading', ... }),
})
```

Map to **downloading** phase (0.5–0.75).

### B5 — `hydrateMissionDocWithProgress` (`ydoc.ts`)

- Async; yield every `chunkSize` slot writes.
- Replace sync `hydrateMissionDoc` call in empty-local path; keep sync wrapper for small payloads if desired.
- Map to **applying** phase (0.75–1.0).

### B6 — Overlay UI (`MissionCreatorPage.tsx`)

- Replace indeterminate `animate-mc-load-bar` with determinate width `value * 100%`.
- Show phase label + optional `"342,891 / 300,000"`.

### C1 — Save upload fix (`useMissionEditor.ts`)

```typescript
await api.post(url, body, {
  timeout: 600_000,
  maxBodyLength: Infinity,
  maxContentLength: Infinity,
  onUploadProgress: ...
})
```

Catch: surface `(e as AxiosError).code` and `.message` when `!resp`.

### C2 — Chunk `assemblePayload` (`compile.ts`)

Yield during large `Object.values` copies so compile % doesn't hit 100% before assembly completes.

### C3 — Vite proxy (`vite.config.ts`)

```typescript
proxy: {
  '/api': { target: '...', changeOrigin: true, timeout: 600_000 },
}
```

### C4 — Optional stretch: gzip upload

`Content-Encoding: gzip` on version POST if body >10 MB — only if straightforward; else defer.

---

## Verification

1. `make test-it`; `npm run build && npm run lint`.
2. **~300k mission:** load shows **real %** through phases; completes (record ms).
3. **Save Version 0.1.x → 201**; GET version returns payload with slot count.
4. Small mission: instant load, save unchanged.
5. Pan ≥55 fps after load (`FpsCounter`).
6. Record §Shipped timings in this file + [t060_fast_initial_load.md](t060_fast_initial_load.md).

---

## Documentation sync (same commit as T-060 tag)

| Doc | Update |
|-----|--------|
| [t060_fast_initial_load.md](t060_fast_initial_load.md) | Status + §Shipped timings; blockers table |
| [t060_1_scale_load_save_completion.md](t060_1_scale_load_save_completion.md) | §Manual verify; §T-060.1.3 + §T-060.1.4 |
| [agent_execution.md](agent_execution.md) | ACTIVE SLICE → **T-066** worker compile |
| [CLAUDE.md](../../CLAUDE.md) §Status | T-060 bullet + 360k acceptance |
| [docs/TAGS.md](../../docs/TAGS.md) | T-060.1 note |
| [feature_inventory.md](feature_inventory.md) | PERF-LOAD-001 / PERF-SAVE-001 acceptance |
| [mission-editor.md](../../frontend/docs/pages/mission-editor.md) | PERF-003/004 determinate + 360k gate |
| [docs/frontend/ROADMAP.md](../../docs/frontend/ROADMAP.md) | T-060 acceptance |
| [docs/AGENT_COMMIT_CHECKLIST.md](../../docs/AGENT_COMMIT_CHECKLIST.md) | T-060.1.4 gate before tag |

---

## Manual verify @ ~360k (2026-06 — user)

### Load — T-060.1.1 **partial pass** (acceptable for ship)

| Criterion | Result |
|-----------|--------|
| Label within 1–2 s | **Pass** — "Reading local save…" visible early |
| Count increases smoothly | **Partial** — stays at **0** for ~30–60 s, then **jumps to ~300k**; remainder loads quickly |
| Stuck blank 0% bar | **Fixed** (T-060.1.1) |
| Wall time | ~30 s–1 min |
| Pan after load | User to confirm ≥55 fps |

**Interpretation:** y-indexeddb applies persisted state as one synchronous `Y.applyUpdate` — `slots.size` cannot tick during the block. T-060.1.1 fixes the **UX symptom** on **legacy v1** (label + pulse, not frozen blank 0%). **T-062.1** v2 chunked restore fixes incremental counts (spec: [`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md)).

### Save — manual verify (2026-06-23)

**Symptom:** Upload reaches **~4%** then `ERR_NETWORK`. E3b direct `:8080` confirmed — **mid-upload failure**, not instant proxy drop @ 0%.

**T-060.1.3 debug report @ 367,526 slots** (mission `70a36667-612f-40c5-ad56-3fb8e0613a17`, semver `0.1.1`):

| Field | Value |
|-------|-------|
| `compiledBytes` / `bytesTotal` | **141,574,630** (~135 MB) |
| `estimatedBytes` | 128,408,282 (~122 MB) |
| `localDocBytes` | 154,517,228 (~147 MB) |
| `uploadRoute` | **`direct`** → `http://localhost:8080/api/v1/missions/.../versions` |
| `bytesLoaded` at failure | **5,573,612** (~5.3 MB) — **3.9%** of total |
| `axiosCode` | `ERR_NETWORK` (no HTTP response) |
| `elapsedMs` | ~4.4 s |
| `phaseAtFailure` | `"preparing"` *(mislabeled — upload had started; **fixed T-060.1.4** — now `'uploading'` on first progress tick)* |

**Interpretation (pre-fix):** Compile + prepare + direct POST succeeded. Connection dropped **early in upload**. Payload is **under** the 256 MB route cap. **Not** the Vite proxy, **not** sync stringify, **not** client timeout (600 s).

**Root cause (proven 2026-06-23 — T-060.1.4 curl sweep):** the **1 MB `GlobalBodyLimit` cap was reaching the version route** because the running API was a **stale `go run` binary** (predating the correct skip; `go run` does not hot-reload). `http.MaxBytesReader` tripped at 1 MB and reset the socket mid-stream → browser `ERR_NETWORK` at **5,573,612 bytes loaded** (TCP send-buffer overshoot past the 1 MB read point — not a 5 MB cap; CMS 5 MB is unrelated). A clean build's `c.FullPath()` already matched `/api/v1/missions/:id/versions`; fresh build + hardened `isMissionVersionPOST` skip returns **201** at 140 MB (~1.2 s). See §T-060.1.4 proven table.

---

## Implemented (T-060.1 code)

- **Part 1 — bulk timing:** `useMissionDoc` keeps the bulk window open through the server hydrate; `endBulkSync(onProgress)` (now async) runs the single coalesced snapshot in the `.finally` after reconcile, then sets `ready`. No more double 300k flush.
- **Part 2 — determinate load:** `docToSnapshotWithProgress` (chunked `slots` conversion) drives the final flush; `hydrateMissionDocWithProgress` (per-chunk INIT_ORIGIN transactions) drives the apply phase; `api.get` `onDownloadProgress` drives the download phase. `loadProgress` ({phase,value,label,done,total}) threads `useMissionDoc → useMissionEditor → MissionCreatorPage` determinate overlay (download 0.15–0.35, apply 0.35–0.55, local 0.55–1.0 after T-060.1.1 reweight; "N / M objects").
- **Part 3 — save upload:** version POST `timeout: 600_000` + `maxBodyLength/maxContentLength: Infinity`; Vite `/api` proxy `timeout`/`proxyTimeout: 600_000`; `compileMissionWithProgress` assembles `editor.slots` via a chunked async copy; the `!resp` catch surfaces axios `code`/`message`.

**Manual verify @ ~300k (2026-06):** bar stuck at **0%** while tab focused for extended period; load eventually completes. **Root cause:** IDB replay dead zone + main-thread block prevents React paint. **Fix → T-060.1.1.** *(Separate issue — T-062.2 fixed alt-tab **full reload overlay** returning on its own in Firefox dev.)*

---

**Pre-T-060.1.1 manual verify:** bar stuck at **0%** while tab focused. **Fixed by T-060.1.1** (see §Manual verify).

---

## Implemented (T-060.1.1 code)

- **`restoring` phase** (`useMissionDoc.ts`): `LoadPhase` includes `'restoring'`; re-weighted bands (restoring 0–15%, download 15–35%, apply 35–55%, local 55–100%); `restoringPhase(done)` with soft curve; rAF poll of `md.entities.slots.size` after `bindStoreToDoc`; `downloadPhase` fixed for unknown `Content-Length`.
- **`yieldToUi`** (`tactical-map/state/yieldToUi.ts`): `scheduler.yield()` ‖ `setTimeout(0)` + `requestAnimationFrame`; used in `docToSnapshotWithProgress` and `hydrateMissionDocWithProgress` (1k reports for first 10k slots, then 5k).
- **Overlay** (`MissionCreatorPage.tsx`): restoring → indeterminate sweep + "N objects restored"; other phases → determinate "N / M objects".
- **Verified:** `npm run build`, `npm run lint`, `make test-it` clean.

---

## Implemented (T-060.1.2 — shipped in T-060 `b1fd25a`)

- **E1/E2 — `preparing` phase + Blob POST:** `SaveProgress.phase` gains `'preparing'`;
  `buildVersionBlob(semver, payload, notes, onProgress)` (`compiler/compile.ts`) streams the
  dominant `editor.slots` array into a `Blob` (yielding via `yieldToUi`), and `saveVersion` POSTs the
  Blob (axios sends it as-is — no second 100MB+ graph stringify). `TopCommandStrip` shows
  "Preparing upload…". *(Keeps the tab responsive but does not fix `ERR_NETWORK` alone.)*
- **E3b — auto direct-upload (the actual fix):** `versionUploadBaseURL(bodyBytes)` in
  `useMissionEditor.ts` overrides the version POST `baseURL` to `http://localhost:8080/api/v1`
  (or `VITE_API_DIRECT_URL`) when `import.meta.env.DEV` and `body.size > 1 MB` and `VITE_API_URL`
  is still the proxy default — so a large save skips the Vite dev proxy (which drops >~2 MB POSTs →
  `ECONNRESET` → `ERR_NETWORK`) and hits the API directly. CORS already allows `:5173` — no backend
  change. One-time dev `console.info` on first bypass; `.env.example` documents `VITE_API_DIRECT_URL`
  / `VITE_API_URL`. Small saves + prod keep the proxy default.
- **Verified:** `npm run build`, `npm run lint`, `make test-it` clean. **Manual verify:** upload ~4% then `ERR_NETWORK` → **T-060.1.3** (measure bytes + debug).

---

## T-060.1.2 — Save upload fixes (E1/E2/E3b — code complete)

**Status:** **shipped in code** — manual verify exposed **mid-upload** failure → **T-060.1.3**.

### Goal

Fix `ERR_NETWORK` on Save Version when the compiled payload is tens–hundreds of MB.

### Root cause (confirmed 2026-06-22)

**Primary:** the **Vite dev proxy** (`localhost:5173/api → :8080`) drops large POST bodies. Live probe:

| Body size | Via Vite proxy (`:5173`) | Direct API (`:8080`) |
|-----------|--------------------------|----------------------|
| ~0.5 MB | 502 | 401 (connection OK) |
| ~1.9 MB | **ECONNRESET** | 401 |
| ~10 MB | 502 | 401 |
| ~48 MB | 502 | 401 |

The Go API accepts payloads >1 MB (integration test passes 2 MB; direct probe returns HTTP responses at 48 MB). **ERR_NETWORK in the browser = no HTTP response**, matching proxy reset — not 413, not backend crash.

**Secondary (mitigated by E2):** synchronous axios `JSON.stringify` — fixed by `buildVersionBlob`.

**E3b (done):** `versionUploadBaseURL()` bypasses Vite proxy for body >1 MB in dev.

**Remaining (T-060.1.3):** mid-upload failure @ ~4% — need **exact `body.size`** and server logs; may be >256 MB, OOM, or axios progress bug.

### Implementation

#### E1 — "Preparing upload…" save phase

Extend `SaveProgress.phase` with `'preparing'`. Yield during request-body construction (chunked stringify of `payload`, then wrap semver/notes) so compile hitting 100% doesn't imply upload has started.

#### E2 — Pre-built body POST

Avoid axios re-stringify of a nested object graph:
- Build final JSON string (or `Blob`) explicitly after chunked assembly.
- POST via `fetch` or axios with `data: bodyString` / `Blob` and existing auth header pattern.
- Keep `timeout: 600_000`, `onUploadProgress` where supported.

#### E3 — Dev bypass (E3b ✅ done; optional `.env.local`)

`frontend/.env.local` (create if missing; **restart Vite after**):
```
VITE_API_URL=http://localhost:8080/api/v1
```
Requires root `.env` `ALLOWED_ORIGINS=http://localhost:5173` (already set). **Verified:** direct API accepts multi-MB POSTs; Vite proxy fails at ~2 MB.

**Code follow-up (E3b):** in `useMissionEditor.ts`, when `import.meta.env.DEV` and `body.size > 1_048_576` and `VITE_API_URL` is still the proxy default (`/api/v1`), override `baseURL` to `http://localhost:8080/api/v1` for the version POST only — so save works without manual `.env.local`.

#### E4 — Optional stretch

`Content-Encoding: gzip` on version POST + gin decompress middleware — only if straightforward.

### T-060.1.2 acceptance

- Save Version 0.1.x → **201** @ ~300k (**default dev path** — auto-bypass proxy; no manual `.env.local` required after E3b).
- GET `/missions/:id/versions/:vid` returns payload with expected slot count.
- Small mission (<500): no regression.
- Load (T-060.1.1): no regression.

### Prevention — don't let this happen again

| Guard | What |
|-------|------|
| **E3b code (required)** | `versionUploadBaseURL()` in `useMissionEditor.ts`: when `import.meta.env.DEV` and body >1 MB and `VITE_API_URL` is still `/api/v1`, POST directly to `:8080` — **never rely on Vite proxy for large uploads**. |
| **Dev startup hint** | One-time `console.info` in dev when axios base is proxy-relative: "Large Mission Creator saves bypass the Vite proxy automatically." |
| **Docs in one place** | [`t060_1_scale_load_save_completion.md`](t060_1_scale_load_save_completion.md) §Root cause + this §Prevention; [`docs/backend/architecture.md`](../../docs/backend/architecture.md) §Dev note; [`frontend/.env.example`](../../frontend/.env.example). |
| **Symptom cheat sheet** | `ERR_NETWORK` @ **0%** in dev → proxy (E3b fixes). `ERR_NETWORK` @ **1%+** → mid-upload (size/logs — T-060.1.3). `413` → body limit. `409` → semver clash. |
| **Optional `.env.local`** | `VITE_API_URL=http://localhost:8080/api/v1` sends **all** API traffic direct (fine for mission makers; not required after E3b). |
| **Do NOT** | Raise proxy timeouts further and call it fixed — probe proved ECONNRESET @ ~2 MB regardless of 600s timeout. |
- `npm run build && npm run lint`; `make test-it`.

### Do not block on

- ≤10 s save @ 1M → T-066 worker compile
- Payload dedup (orbat vs editor.slots) → **shipped T-062.1.1**

---

## After T-060.1.4 (shipped in T-060 `b1fd25a`)

**T-061..T-067:** mission-layer scale (typed-array → incremental bindings → …). **Eden T-068+.** **T-110:** optional terrain base + deltas — [`t110_terrain_base_mission_layers.md`](t110_terrain_base_mission_layers.md).

---

## T-060.1.3 — Save observability + measured size (**shipped**)

**Status:** **O1–O6 complete + manual observability verify passed** @ 367k — exact bytes, route, and axios code captured; failure fully diagnosed (see §Manual verify Save). **Does not fix mid-upload** — that is **T-060.1.4**.

**Implemented (O1–O6):** `lib/missionSize.ts` (`formatBytes`, `estimateCompiledBytes`, `getLocalDocBytes`, `SERVER_VERSION_BODY_LIMIT`); rich `SaveProgress` + `SaveDebugReport` + `SaveResult.debug` with per-phase `console.debug`/failure `console.error`; pre-upload 256 MB gate (exact-bytes message, no POST); `resolveVersionUpload` yields route label (`proxy`/`direct`/`configured`); `onUploadProgress` falls back `bytesTotal = e.total ?? body.size` (the "4%" red herring). UI: toolbelt **SZ** (debounced estimate), Save dialog pre-save estimate + LOC, exact MB during prepare, `loaded / total MB` + route chip during upload, amber >200 MB, copyable **Debug details** on failure. Backend: `CreateVersion` logs entry content-length + exit status/bytes/duration (413 mapping verified). `npm run build`/`lint` + `make test-it` clean.

### Revised diagnosis (2026-06-23 — updated after T-060.1.3 manual verify)

| Observation | Meaning |
|-------------|---------|
| Save dialog shows **Uploading… 4%** | Compile + prepare completed; upload **started** |
| Then `ERR_NETWORK` | Connection dropped **mid-upload** — not instant proxy failure @ 0% |
| `uploadRoute: direct` | **Confirmed** — E3b bypass working |
| `compiledBytes: 141574630` | **135 MB** — under 256 MB cap; pre-gate did not block |
| `bytesLoaded: 5573612` | Server/client read ~**3.9%** before reset |

**Ruled out:** Vite proxy, sync stringify, 256 MB cap, 600 s timeout, misleading progress % (total fallback correct).

**Leading hypotheses (T-060.1.4 — RESOLVED 2026-06-23):** see §T-060.1.4 proven table — **stale `go run` binary** + 1 MB global wrap; not 256 MB cap, not OOM, not 5 MB CMS limit.

**Not batch upload (T-060 era):** current API is one `POST /missions/:id/versions` with full `json_payload`. Multipart/chunk upload remains **deferred**. **T-062.1.1 shipped Option A** — orbat dedup only (editor-only Save; Go derives ORBAT); not a frontend hack.

### Goal

Know **exactly** how big the mission is (local + compiled upload bytes) before and during save; show it frictionlessly in the UI; log enough to diagnose mid-upload failures in one attempt.

### Implementation

#### O1 — `missionSize.ts` (new helper module)

`frontend/src/features/mission-creator/lib/missionSize.ts`:

- `formatBytes(n: number): string` — macOS Finder style (`187.4 MB`, `1.2 GB`).
- `estimateCompiledBytes(state: MapSnapshot): number` — sample 20 slots, extrapolate `editor.slots` only (~1.0× since **T-062.1.1**; was ~1.35× with duplicate `orbat[]`); fast, no full compile.
- `getLocalDocBytes(md: MissionDoc): number` — `Y.encodeStateAsUpdate(md.doc).byteLength` (IndexedDB / CRDT footprint).
- `SERVER_VERSION_BODY_LIMIT = 256 << 20` — shared constant with pre-gate message.

#### O2 — Rich `SaveProgress` + `SaveResult`

Extend types in `useMissionEditor.ts`:

```typescript
export interface SaveProgress {
  phase: 'compiling' | 'preparing' | 'uploading'
  value?: number
  // T-060.1.3 observability
  slotCount?: number
  compiledBytes?: number      // exact after buildVersionBlob
  estimatedBytes?: number     // fast estimate before compile (optional)
  localDocBytes?: number
  bytesLoaded?: number
  bytesTotal?: number
  uploadRoute?: 'proxy' | 'direct' | 'configured'
  uploadUrl?: string          // resolved base + path (no token)
  elapsedMs?: number
}

export interface SaveResult {
  ok: boolean
  error?: string
  debug?: SaveDebugReport     // always populated on failure; optional on success
}
```

`SaveDebugReport`: all fields above + axios `code`, `message`, `response?.status`, phase at failure, timestamp.

Log every phase to `console.debug('[mission-save]', …)` in dev; on failure also `console.error` with full report.

#### O3 — Pre-upload size gate

After `buildVersionBlob`, **before** `api.post`:

- If `body.size > SERVER_VERSION_BODY_LIMIT` → return `{ ok: false, error: 'Compiled payload is X MB — server limit is 256 MB. …' }` with debug report. **Do not upload.**
- If `body.size > 200 MB` → warn in Save dialog (amber) but allow proceed.

#### O4 — Save dialog UI (TopCommandStrip)

macOS-style, frictionless:

- **Before save starts:** show `~187 MB estimated · 360,284 objects` (estimate + slot count).
- **During prepare:** `Preparing… 187.4 MB` (exact `body.size` once known).
- **During upload:** `Uploading 8.2 / 187.4 MB (4%)` + route chip `direct → :8080`.
- **On error:** expandable **Debug details** mono block (copy-to-clipboard button) with full `SaveDebugReport` — user can paste into chat/issues.

#### O5 — Bottom toolbelt size readout (macOS frictionless)

Next to existing `OBJ` / `SEL` in [`BottomToolbelt.tsx`](../../frontend/src/features/mission-creator/layout/BottomToolbelt.tsx):

```
SZ 187 MB ~   // estimated compiled size; memoized; refresh when slot count changes (debounced 500ms)
LOC 42 MB     // local Y.Doc bytes (optional second line or tooltip)
```

Use `formatBytes` + `estimateCompiledBytes`; tooltip: "Estimated server save size · Local IndexedDB size".

#### O6 — Backend request logging (minimal)

In `CreateVersion` or route middleware: log `Content-Length` (if present), mission id, semver, duration, outcome. On `MaxBytesError` ensure **413 JSON** before connection drop where possible.

#### O7 — Fix path after measurement

Once exact `compiledBytes` is known:

- If **> 256 MB** → document: reduce duplication (T-062 strips redundant orbat), or raise `MISSION_VERSION_MAX_BODY_BYTES` env (dev only).
- If **< 256 MB** but mid-upload fail → try `fetch` + `ReadableStream` or split read; check API logs during upload; verify CORS on long POST.

### T-060.1.3 acceptance

- [x] Bottom toolbelt shows **SZ** estimate within ~10% of actual compiled size @ 360k.
- [x] Save dialog shows **exact MB** after prepare, upload progress as `loaded / total MB`.
- [x] On failure, debug panel shows bytes, route, URL, phase, axios code — **no more bare ERR_NETWORK**.
- [x] Payload < 256 MB confirmed (135 MB) — pre-gate did not block.
- [x] Failure **fully diagnosed** (exact bytes + route + axios code) @ 367k.
- [x] Save → **201** @ ~367k — **shipped T-060.1.4** (curl 140 MB → 201; browser ~142 MB → 201, 2026-06-23).
- [x] Load unchanged; small mission save unchanged.

---

## T-060.1.4 — Fix mid-upload socket reset @ ~135 MB (**shipped**)

**Status:** **shipped** in `b1fd25a` (2026-06-23) — root cause **proven** with curl + server log; browser Save @ ~367k/~142 MB → **201**. The mid-upload
`ERR_NETWORK` was the **1 MB global body cap reaching the version route** (the `GlobalBodyLimit`
skip not applying — most likely a **stale `go run ./cmd/api` binary**; `go run` does not hot-reload,
and the user's running API predated the correct skip). `http.MaxBytesReader` tripped at 1 MB and
reset the connection mid-stream, which the browser reported as `ERR_NETWORK` at ~5 MB buffered (the
TCP send-buffer overshoot past the 1 MB server read point — exactly the locked
`bytesLoaded: 5,573,612`). Payload (135 MB) was never near the 256 MB route cap.

### Proven (2026-06-23, curl against `make api`)

| Body | Original running binary (stale, skip not applied) | Fresh build + hardened skip |
|------|---------------------------------------------------|-----------------------------|
| 2 MB | **400** "semver and payload are required" (1 MB `MaxBytesReader` trip → generic bind error) | **201** (28 ms) |
| 10 MB | **reset mid-upload** (`size_upload` cut to ~4.5 MB) | **201** (91 ms) |
| 140 MB | **reset mid-upload** (~4.5 MB) | **201** (1.2 s, clean `content_length=146800700 status=201`) |

A `gin` probe confirmed `c.FullPath()` correctly returns `/api/v1/missions/:id/versions` in the
global middleware on a **clean build** — so the on-disk skip was already correct; the failure was a
**stale binary**. The hardened skip + production-like IT make the regression impossible to ship
again and catch it in CI.

### Implemented

- **`isMissionVersionPOST(c)`** in [`bodylimit.go`](../../internal/middleware/bodylimit.go): keeps the
  `FullPath()` suffix match **and** adds a concrete-URL-path fallback (`/…/missions/<id>/versions`),
  so the 1 MB global wrap can never silently apply even if `FullPath()` is empty/unexpected.
- **Production-like integration test** [`missions_bodylimit_integration_test.go`](../../internal/handlers/missions_bodylimit_integration_test.go):
  `setupITProd` mounts `GlobalBodyLimit(MaxJSONBody)` like `cmd/api/main.go` (the blind spot —
  `setupIT` used a bare router). Asserts 2 MB & 3.5 MB version POST → **201** (global cap skipped) and
  a 5 MB body over a pinned 4 MB route cap → **413 JSON** (`status=413 over_limit`). Plus a
  [`bodylimit_test.go`](../../internal/middleware/bodylimit_test.go) unit test for the route-pattern
  match and the URL-path fallback.
- **`phaseAtFailure` fix** ([`useMissionEditor.ts`](../../frontend/src/features/mission-creator/hooks/useMissionEditor.ts)):
  `report.phaseAtFailure = 'uploading'` on the first `onUploadProgress` tick, so a mid-upload failure
  no longer mislabels as `'preparing'`.
- **Repro script** [`scripts/mission-version-upload-repro.sh`](../../scripts/mission-version-upload-repro.sh):
  dev-login → create mission → curl `--data-binary` at 2/10/140 MB; splits server-side vs browser-side.
- **B-4 not needed:** the route-level `BodyLimit` cap already maps `*http.MaxBytesError` → **413 JSON**
  (verified in IT); the 135 MB body binds via `ShouldBindJSON` in ~1.2 s with no OOM. No streaming rewrite.

**For the user:** if Save still fails after pulling this, **restart `make api`** (the bug was a stale
binary) and watch for `CreateVersion: mission=… content_length=141574630 … status=201`.

---

### (original task framing — kept for reference)
T-060.1.3 eliminated guesswork; this slice fixes Save → **201** @ ~367k or proves root cause with server logs + curl.

### What we know (locked — do not re-litigate)

| Fact | Source |
|------|--------|
| Compiled payload **141,574,630 bytes** (~135 MB) | T-060.1.3 `SaveDebugReport` |
| Upload route **`direct`** to `:8080` | Same |
| Failure at **5,573,612 bytes loaded** (~3.9%) | Same |
| **`ERR_NETWORK`** — no HTTP response | Same |
| **Not** Vite proxy, **not** sync stringify, **not** 256 MB cap | E3b + E1/E2 + pre-gate |
| **Not** a built-in **5 MB** limit on version POST | Code review — 5 MB is CMS uploads only |
| `setupIT()` does **not** mount `GlobalBodyLimit` | Integration-test blind spot |

### Goal

**Save Version → 201** @ ~367k / ~135 MB on dev stack (`make api && make web`), **or** fully proven root cause with API terminal log + curl reproduction.

### Implementation

#### F1 — Correlate server log (mandatory first step)

On Save, watch `make api` for:

```
CreateVersion: mission=70a36667-... content_length=141574630
```

| Log outcome | Next action |
|-------------|-------------|
| **No entry log** | Auth/CORS/network before handler — trace middleware |
| **Entry + `status=413 over_limit`** | MaxBytesReader — verify global vs route wrap |
| **Entry + silence + RST** | Mid-read abort — F2/F3 |
| **Entry + `status=201`** | Client anomaly — fetch fallback |
| **Process restart** | OOM — F4 streaming bind |

#### F2 — Harden `GlobalBodyLimit` skip

[`internal/middleware/bodylimit.go`](../../internal/middleware/bodylimit.go): keep `FullPath()` suffix match; add **URL path fallback** — e.g. `POST` and path matches `/missions/*/versions` — so the 1 MB global wrap cannot accidentally apply when `FullPath()` is empty.

#### F3 — Production-like integration test

Extend test router (new helper or `setupITWithMiddleware`) to mount `GlobalBodyLimit(MaxJSONBody)` like [`cmd/api/main.go`](../../cmd/api/main.go). POST a **2–10 MB** version body → assert **201**, not connection reset. Optionally assert **413** on a body > 256 MB.

#### F4 — Backend body handling for 100MB+ payloads

If logs show mid-read RST or OOM after full read:

- Stream request body to **temp file** instead of monolithic `ShouldBindJSON` into `json.RawMessage` (or use `json.Decoder` with size guard).
- Log **bytes read** incrementally during bind.
- Ensure `MaxBytesError` returns **413 JSON** before connection teardown where possible.

#### F5 — Frontend observability fix (minor)

[`useMissionEditor.ts`](../../frontend/src/features/mission-creator/hooks/useMissionEditor.ts): call `logPhase('uploading')` on first `onUploadProgress` so `phaseAtFailure` is accurate.

#### F6 — curl isolation test

After dev-login token, POST compiled JSON directly:

```bash
curl -v -X POST "http://localhost:8080/api/v1/missions/<id>/versions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  --data-binary @compiled.json
```

curl dies mid-upload → server/middleware. curl **201** → browser/axios/memory.

### DO NOT (this slice)

- Re-implement E1/E2/E3b or T-060.1.3 observability
- Add batch/chunk upload (multipart **deferred** — T-062.1.1 shipped **orbat dedup only**)
- Raise 256 MB cap (payload is 135 MB — already under)
- **Edit documentation** — **Cursor (Composer 2.5)** owns all doc sync; Claude Code reads specs only
- Commit until user says. Tag **T-060** = single commit T-060..**T-060.1.4** after Save → **201**

### T-060.1.4 acceptance

- [x] Version POST → **201** @ **140 MB** via curl (`--data-binary`), clean `content_length`/`status=201` log — the 135 MB save's size class.
- [x] `make test-it` includes a production-like router with `GlobalBodyLimit` mounted + multi-MB version POST (201) + over-cap (413).
- [x] `npm run build && npm run lint` clean; `go build ./...` + `gofmt` clean.
- [x] Load + small-mission save unchanged (no touched code on those paths; small saves keep the proxy route).
- [x] Docs synced (see §Documentation sync).
- [x] **Browser** Save Version → **201** @ ~367k (semver **0.1.3** / **0.1.4**, ~142 MB upload, `direct → :8080`) — **2026-06-23 manual verify passed**.

### Prevention (updated)

| Guard | What |
|-------|------|
| **GlobalBodyLimit skip + URL fallback** | Version route never gets 1 MB wrap |
| **Production-like IT** | CI catches middleware regression |
| **Server log correlation** | `CreateVersion` entry line splits handler vs network |
| **curl repro** | Isolates browser from Go |
| **Save orbat dedup** | **T-062.1.1** ✅ — editor-only Save + Go ORBAT derive (multipart upload deferred) |

---

> **Historical archive — prompts below are pre-ship (T-060 shipped `b1fd25a`, 2026-06-23). Preserved for archaeology; do not execute.**

## Claude Code prompt (copy-paste — T-060.1.4)

Use a **fresh** Claude Code chat. Read §T-060.1.4 + [`agent_execution.md`](agent_execution.md) §ACTIVE SLICE + §Manual verify Save (debug JSON).

```
T-060.1.4 — Fix mid-upload ERR_NETWORK @ ~367k / ~135 MB

Read t060_1_scale_load_save_completion.md §T-060.1.4 + §Manual verify Save + agent_execution.md §ACTIVE SLICE.

## What we know (STOP GUESSING — locked)

T-060.1.3 SHIPPED. Manual verify @ 367,526 slots captured SaveDebugReport:
- compiledBytes: 141574630 (~135 MB)
- uploadRoute: "direct" → http://localhost:8080/api/v1/missions/70a36667-612f-40c5-ad56-3fb8e0613a17/versions
- bytesLoaded at failure: 5573612 (~3.9%)
- axiosCode: ERR_NETWORK (no HTTP response)
- elapsedMs: ~4412

NOT: Vite proxy (direct route), sync stringify (E1/E2), 256 MB cap (135 MB < 256 MB), 600s timeout.

Partner theory review: backend mid-stream reset is PLAUSIBLE. "5 MB MaxBytesReader default" is WRONG for this route — 5 MB is CMS file uploads only (cms.go). Relevant limits: 1 MB global (GlobalBodyLimit, skipped via FullPath suffix), 256 MB route (BodyLimit). Client bytesLoaded can overshoot server read (TCP buffer) — 5.5 MB ≠ 5 MB cap. setupIT() does NOT mount GlobalBodyLimit — production middleware untested in CI.

## DO NOT
- Re-implement E1/E2/E3b or T-060.1.3 observability
- Add batch/chunk upload (multipart **deferred** — T-062.1.1 shipped orbat dedup only)
- **Edit any documentation files** — Cursor (Composer 2.5) owns all doc sync; read docs only
- Commit until I say. Tag T-060 = single commit T-060..T-060.1.4 after Save → 201

## PART 0 — Server log (do this FIRST on next Save attempt)
Watch make api terminal when clicking Save. Look for:
  CreateVersion: mission=70a36667-... content_length=141574630
Record whether entry log, 413, 201, or process restart appears. Paste log into commit notes.

## PART 1 — Harden GlobalBodyLimit skip (bodylimit.go)
- Keep FullPath() suffix match on MissionVersionRoute
- Add URL path fallback: POST + path matches /missions/*/versions pattern
- Unit or integration test proving version POST is NOT wrapped at 1 MB

## PART 2 — Production-like integration test
- New test helper mounting GlobalBodyLimit(MaxJSONBody) like cmd/api/main.go
- POST 2–10 MB version body → 201
- Optional: >256 MB → 413 JSON (not silent RST)

## PART 3 — Fix phaseAtFailure bug (useMissionEditor.ts)
- Call logPhase('uploading') on first onUploadProgress tick

## PART 4 — Backend body handling (if logs show mid-read RST or OOM)
- Stream large POST body to temp file OR incremental read with byte logging
- Ensure MaxBytesError → 413 JSON before connection drop
- CreateVersion already logs entry content_length + exit status — extend with bytes-read if helpful

## PART 5 — curl isolation
- Document/script: curl --data-binary @compiled.json to :8080 with Bearer token
- If curl fails same way → server fix. If curl 201 → consider fetch() fallback.

## DO NOT — documentation (Cursor owns docs)
- Do NOT edit CLAUDE.md, ROADMAPs, TAGS, feature_inventory, mission-editor.md, agent_execution.md, or t060_1 spec files
- Return a short verify report (curl output, test-it, browser Save result) so Cursor can sync docs

VERIFY
- make test-it; npm run build && npm run lint
- ~367k Save → 201 OR fully proven with server log + curl
- Load partial pass unchanged; small mission save unchanged
- Do NOT commit until I say
```

### Chat guidance

**Use a fresh Claude Code chat.** T-060.1.3 observability is complete; T-060.1.4 is a focused backend/middleware fix with server-log correlation. Prior chat context about proxy/stringify is historical — do not re-open.

---

## Claude Code prompt (copy-paste — T-060.1.3 — historical)

Use the **same Claude Code chat** (uncommitted stack) or fresh. Read §T-060.1.3 + [`agent_execution.md`](agent_execution.md) §ACTIVE SLICE.

```
T-060.1.3 — Save observability + measured size @ ~360k

Read t060_1_scale_load_save_completion.md §T-060.1.3 + agent_execution.md §ACTIVE SLICE.

## What we know (stop guessing)

E1/E2/E3b are DONE (buildVersionBlob, preparing, versionUploadBaseURL direct :8080).
User still gets ERR_NETWORK but upload reaches ~4% first → mid-upload failure, NOT instant proxy drop.

We do NOT know exact payload bytes yet. Implement measurement + debug BEFORE more upload hacks.

## DO NOT
- Re-implement E1/E2/E3b
- Add batch/chunk upload (multipart **deferred** — T-062.1.1 shipped orbat dedup only)
- Commit until I say. Tag T-060 = T-060..T-060.1.3 single commit after Save → 201 OR fully diagnosed.

## PART 1 — missionSize.ts
- formatBytes (macOS style: 187.4 MB)
- estimateCompiledBytes(state) — sample 20 slots, extrapolate with orbat factor
- getLocalDocBytes(md) — Y.encodeStateAsUpdate byteLength
- SERVER_VERSION_BODY_LIMIT = 256 << 20

## PART 2 — Rich SaveProgress + SaveResult + logging
- Extend SaveProgress: slotCount, compiledBytes, estimatedBytes, localDocBytes,
  bytesLoaded, bytesTotal, uploadRoute, uploadUrl, elapsedMs
- SaveDebugReport on failure (always); console.debug each phase; console.error full report on fail
- Pre-gate: if body.size > 256MB → error BEFORE upload with exact size in message

## PART 3 — UI (macOS frictionless)
- BottomToolbelt: SZ ~187 MB (estimate, debounced on slot count) + LOC tooltip for local bytes
- TopCommandStrip Save dialog:
  - Before: ~estimate · N objects
  - Preparing: exact MB
  - Uploading: loaded / total MB (%) + route chip (direct → :8080)
  - Error: expandable Debug details + Copy button (full SaveDebugReport JSON)

## PART 4 — Backend
- Log Content-Length + mission id on POST /missions/:id/versions (start + outcome + duration)
- Ensure MaxBytesError → 413 JSON where possible

## PART 5 — Diagnose + fix if obvious
After implementing, if compiledBytes > 256MB → document in error (orbat+editor duplication).
If < 256MB but still fails → check API log during upload; consider fetch() fallback with same auth.

## PART 6 — DOCS
Sync: t060_1 §T-060.1.3, agent_execution, feature_inventory PERF-SAVE-001,
mission-editor.md, CLAUDE.md, ROADMAP, TAGS.

VERIFY
- make test-it; npm run build && npm run lint
- ~360k: toolbelt SZ within ~10% of actual; save dialog shows exact MB
- Failure shows debug panel (not bare ERR_NETWORK)
- Save → 201 OR we know exact bytes + server log reason
- Load + small mission unchanged
```

---

## Claude Code prompt (copy-paste — T-060.1.1 — shipped in code)

Use a **fresh** Claude Code chat (see §Chat guidance below). Read this file §T-060.1.1 and [`agent_execution.md`](agent_execution.md) §ACTIVE SLICE first.

```
T-060.1.1 — Fix stuck-at-0% load bar @ ~300k (IDB restoring phase + paint-friendly progress)

Read Design_Docs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md (§T-060.1.1 + §Implemented).

Context: T-060 + T-060.1 code is landed (uncommitted). T-060.1 fixed bulk-timing, chunked download/apply/local progress, and save-upload timeouts. Manual verify @ ~300k PARTIAL FAIL: overlay stuck at 0% for minutes while tab focused (Firefox "slowing down"); load eventually completes — alt-tab coincidence, not a focus bug. Root cause: y-indexeddb replay into Y.Doc runs BEFORE persistence.once('synced') with zero reportLoad calls; main thread blocked so React never paints.

Do NOT commit until ~360k load shows motion within 1–2 s AND Save → 201. Tag T-060 (single commit: T-060 + T-060.1 + T-060.1.1).

PART 1 — restoring PHASE (useMissionDoc.ts)
1. Extend LoadPhase with 'restoring'; add restoringPhase(done) — weight 0–0.15
2. After bindStoreToDoc, start rAF poll: md.entities.slots.size → reportLoad(restoringPhase(count)); label "Reading local save…"
3. Cancel poll on synced or unmount
4. Re-weight: restoring 0–15%, download 15–35%, apply 35–55%, local 55–100% (update downloadPhase/applyPhase/localPhase)
5. Fix downloadPhase when Content-Length unknown: don't use frac(loaded, 0) → 1

PART 2 — PAINT-FRIENDLY YIELDS
1. New yieldToUi.ts: scheduler.yield() || setTimeout(0) + requestAnimationFrame
2. Use in docToSnapshotWithProgress, hydrateMissionDocWithProgress (replace bare setTimeout)
3. Wrap reportLoad to await yieldToUi() before setLoadProgress
4. Optional: report every 1000 slots for first 10k, then 5000

PART 3 — OVERLAY UX (MissionCreatorPage.tsx)
1. restoring + unknown total: "N objects restored" + indeterminate pulse CSS on bar
2. When done+total known: "N / M objects" determinate bar

PART 4 — DOCS (same pass, no separate commit)
Sync §Documentation sync table + agent_execution ACTIVE SLICE → T-063 after T-062.2 ship; flip acceptance checkboxes when code lands.

VERIFY
- make test-it; npm run build && npm run lint
- ~300k mission (return visit, IDB cache): within 1–2 s "Reading local save…" + count increasing; bar moves through all phases; record wall time
- Save Version 0.1.x → 201; GET version returns payload
- Small mission (<500): unchanged
- Pan ≥55 fps after load (FpsCounter)
- Do NOT commit until I say
```

### Chat guidance

**Use a fresh Claude Code chat.** The prior T-060.1 chat completed that slice; T-060.1.1 is a focused follow-up with a different root cause (IDB dead zone). A fresh chat avoids conflating completed work with the new fix and keeps context tight. Same repo / uncommitted tree — no need to re-implement T-060.1.
