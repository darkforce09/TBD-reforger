# T-062.1.1 — ORBAT payload dedup on Save Version (Option A)

**Status:** **shipped + verified** — automated gates + manual @ 367,529 objects: **~94.8 MB estimated** compiled (was ~141 MB pre-dedup, **~33% smaller**); Save dialog + size readout confirmed 2026-06-24.  
**Git tag on ship:** **T-062.1.1** (`4baf5fa`)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t062_1_idb_streaming_load.md](t062_1_idb_streaming_load.md) · [t060_1_scale_load_save_completion.md](t060_1_scale_load_save_completion.md) §Payload dedup

**Prerequisites:** T-062.1 shipped (`4ad27fe`).

---

## Problem (pre-T-062.1.1)

Save Version compiled every slot **twice** in one POST body:

| Block | Contents |
|-------|----------|
| `orbat[]` | Nested factions → squads → `{ role, loadout, tag }` per slot |
| `editor.slots[]` | Full slot objects (id, position, stance, …) |

@ ~360k stress test: **~141 MB** compiled ([`t060_1`](t060_1_scale_load_save_completion.md) §T-060.1.3). Hydrate already uses `editor` only; `orbat[]` was dead weight on Save — except Event Hub auto-ORBAT read only top-level `orbat[]`.

Real missions (~128 ORBAT slots) are tiny; this slice targets **future object-heavy saves** (props, vehicles, sandbags) and **256 MB headroom** at scale.

---

## Goal

**Option A (locked):** Omit `orbat[]` from Save payload; Go **derives** ORBAT from `editor` when `orbat` is missing/empty. Export keeps full superset with `orbat`.

**Target @ ~360k:** ~25–30% smaller POST (~100–110 MB vs ~141 MB); Save → **201** unchanged.

---

## Shipped implementation

### Backend

| File | Role |
|------|------|
| `internal/services/mission_payload.go` | `ParseOrbatTemplate`: legacy `orbat[]` wins; else `deriveOrbatFromEditor` (mirrors `compile.ts` order) |
| `internal/services/mission_payload_test.go` | Unit: legacy wins, editor-only, empty, MED tag/order |
| `internal/handlers/missions_orbat_integration_test.go` | IT: editor-only version → attach event (no explicit orbat) → GET orbat |
| `internal/handlers/events.go` | Type aliases to services templates; `parseOrbatTemplate` → thin wrapper |

**Derivation order:** faction array → `squadIds` → resolve squad → `slotIds` → resolve slots → sort by `index` asc → `{ role, loadout: "", tag }`.

### Frontend

| File | Role |
|------|------|
| `compiler/compile.ts` | `CompileOptions { omitOrbat?, chunkSize? }`; `orbat?` optional on `MissionPayload`; save path skips orbat; export `compileMission` unchanged |
| `useMissionEditor.ts` | `saveVersion`: `{ omitOrbat: true }`; over-limit error reworded |
| `lib/missionSize.ts` | Estimate factor **1.35 → 1.0** (editor-only save) |

**Unchanged:** `buildVersionBlob` (orbat-agnostic `{ editor, ...rest }`), hydrate, v2 IDB, Export.

---

## Contract

| Path | Payload |
|------|---------|
| **Save (new)** | `editor` block present; **no `orbat` key** (or empty) |
| **Save (legacy)** | `orbat[]` + `editor` — stored as-is; legacy orbat wins on read |
| **Export** | Full superset with `orbat[]` (`compileMission`) |
| **Event attach** | `ParseOrbatTemplate` → explicit orbat OR derive from editor |

---

## Acceptance (T-062.1.1)

| Check | Result |
|-------|--------|
| `make test-it` | **Pass** — incl. `TestEditorOnlyOrbatDerivationIntegration` |
| `go test ./internal/services/...` | **Pass** |
| FE build + lint | **Clean** |
| Legacy explicit-orbat event attach | **Pass** (unchanged) |
| Editor-only → event ORBAT | **Pass** — 2 squads / 4 slots, index order, MED tag |
| Manual Save @ ~360k | **Pass** — 367,529 objects; **~94.8 MB estimated** (was ~141 MB; **~33% smaller**) |

---

## Out of scope

- Multipart/chunked upload (Option B)
- schemaVersion 2 compact `editor.slots`
- T-063 spatial index → **shipped** [`t063_spatial_index.md`](t063_spatial_index.md)

---

## After T-062.1.1

- **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**

---

## Documentation sync (Cursor — this commit)

`agent_execution.md`, `ROADMAP.md`, `CLAUDE.md`, `mission-editor.md`, `feature_inventory.md`, `TAGS.md`, `docs/frontend/ROADMAP.md`, `docs/AGENT_COMMIT_CHECKLIST.md`, `docs/backend/architecture.md`, spec footers (`t062_1`, `t062`, `t062_2`, `t060_1`, `t057`–`t061`, `t070`).
