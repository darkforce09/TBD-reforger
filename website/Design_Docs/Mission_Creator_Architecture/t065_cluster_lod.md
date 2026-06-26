# T-065 — Cluster / LOD when zoomed out (map IconLayer)

**Status:** **shipped** — T-065 + T-065.1 + T-065.2; FE build/lint clean. **Product call (2026-06):** good enough — daily edit zoom stays detail @ ~160 fps; extreme-zoom clusters acceptable; no further cluster perf polish unless regression.  
**Git tag on ship:** **T-065**  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t061_drag_move_hotfix.md](t061_drag_move_hotfix.md) · [t063_spatial_index.md](t063_spatial_index.md)

**Prerequisites:** T-064 shipped (`8838e58`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~367k slots).

---

## In one sentence

**At extreme zoom-out on mega missions, show pan-stable cluster discs; at normal edit zoom keep all 367k rings @ ~160 fps — clustering is infrastructure for 1M+, not a fix for 367k detail mode.**

---

## Problem (revised after manual verify)

Original premise: @ min zoom, Deck draws ~367k `IconLayer` markers wastefully → cluster for fps.

**Manual verify (2026-06) disproved this @ 367k for detail mode:**

| Mode | Zoom | Pan FPS | Notes |
|------|------|---------|-------|
| Detail (367k rings) | -2 (default) | **~160** | T-061 pan-stable `slotIconCache` |
| Detail (367k rings) | > -2 (zoomed in) | **~160** | Zero lag |
| Cluster (T-065.1, viewport bbox) | ≤ -2 | **Stutter** (~48 fps) | Per-pan `setMarkers` — fixed in T-065.2 |

**T-065.1 regression root cause:** `useClusterIconLayer` re-queried viewport bbox + `setMarkers` every pan → new `IconLayer` data each frame (violates T-061 pan-stability).

**T-065.2 fix:** full-terrain `getClusterMarkers` module cache + `ZOOM_CLUSTER_MAX = -4` so default load never enters cluster band.

**Eden parity (deferred nuance):** [`eden/ui_anatomy.md`](eden/ui_anatomy.md) — group icons stacked when zoomed out. Geo clusters at extreme zoom only for v1.

---

## Goal

- **367k @ default/edit zoom (-2):** detail mode (all rings) @ **~160 fps** — no cluster overlap.
- **Mega missions @ zoom ≤ -4:** cluster discs, pan-stable (layer data unchanged on pan).
- **Selected slots:** visible as individual highlighted icons in cluster band.
- **Out of scope:** Y.Doc/schema/backend, T-066 worker, T-067 chunks, squad-semantic clustering v1, further cluster perf tuning unless regression.

---

## Implementation (working tree — T-065 / T-065.1 / T-065.2)

| File | Role |
|------|------|
| `state/constants.ts` | `ZOOM_DETAIL_MIN = -2` (doc/default open zoom); `ZOOM_CLUSTER_MAX = -4`; `CLUSTER_SLOT_THRESHOLD = 500` (= `BULK_SELECT_CAP`, **not** outliner `VIRTUAL_SLOT_THRESHOLD` 50) |
| `state/slotClusterIndex.ts` | supercluster singleton; terrain-normalized coords; lazy `ensureForest()`; **`getClusterMarkers` / `getClusterMarkersVersion`** pan-stable full-terrain cache (T-065.2) |
| `state/slotIconCache.ts` | cluster index wired in mutators when `dense.length > 500` |
| `layers/useClusterIconLayer.ts` | Cluster discs; reads module cache; `useMapStore(iconCacheVersion)`; no rAF/useState/bbox (T-065.2) |
| `layers/useIconLayer.ts` | `detail` gate — cluster mode renders selected only |
| `TacticalMap.tsx` | `clusterMode = slotCount > 500 && zoom <= ZOOM_CLUSTER_MAX`; no ResizeObserver/bbox |
| `tools/useSelectTool.ts` | Cluster pick / drill-in; no full rbush on cluster-mode click |
| `view/useOrthographicView.ts` | Default zoom `-2`; `flyTo(target, zoomDelta)` |

**Dependency:** `supercluster@^8`, `@types/supercluster@^7`

---

## Interaction contract

- Detail mode (zoom > -4 or ≤500 slots): T-063 pick, T-061 drag, T-053 Ctrl-toggle — unchanged.
- Cluster band (zoom ≤ -4, >500 slots): click/dbl-click cluster → flyTo centroid + zoom +1; no member selection.
- Cluster band: `onPointerDown` skips full rbush (drag from hidden slot blocked — zoom in to edit).
- Marquee: full rbush `pickRect` unchanged.
- `slot-icons` / cluster layers: `pickable: false`.

---

## Acceptance (ship gate)

| Check | Bar | Status |
|-------|-----|--------|
| Default open @ 367k (zoom -2) | Rings, ~160 fps pan | ✅ detail path |
| Zoom ≤ -4 | Discs; pan-stable (no per-pan layer rebuild) | ✅ T-065.2 code |
| Zoom > -4 | Rings; ~160 fps; T-063 parity | ✅ narrow band |
| ≤500-slot mission | Rings at all zooms | ✅ threshold |
| Outliner + Asset Browser | Unchanged (T-064) | ✅ |
| `npm run build` + `npm run lint` | Clean | ✅ |

---

## After T-065 ship

- **Shipped:** T-065 cluster/LOD — [`t065_cluster_lod.md`](t065_cluster_lod.md) (`845bfb2`)
- **T-067** ✅ shipped — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Next:** Eden **T-068+**
- Eden **T-068+**

---

## Documentation sync (Cursor — pending T-065 ship)

On commit: `CLAUDE.md` §Status, `agent_execution.md`, `ROADMAP.md`, `feature_inventory.md` (PERF-CLUSTER-001), `TAGS.md`, `docs/AGENT_COMMIT_CHECKLIST.md`, `mission-editor.md`, spec footers (`t064`, `t063`, …).

---

## Claude Code prompt archive

Historical — T-065/T-065.1/T-065.2 prompts completed in working tree. Do not re-run unless regressing cluster or detail perf.
