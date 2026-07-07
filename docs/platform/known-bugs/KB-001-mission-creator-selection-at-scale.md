# KB-001 — Mission Creator: selection & copy/paste break at extreme slot counts

| | |
|---|---|
| **Status** | Known · **Deferred** (out of realistic scale) |
| **Severity** | Low — only manifests far beyond the usable envelope |
| **Area** | Mission Creator — selection / picking / render subsystem |
| **Discovered** | 2026-07-07, during the T-145 F3.1 operator browser gate @ **517,968 objects** |
| **Perf** | Not a perf bug — 60 fps sustained at the time |

## Why this is deferred

500k slots is a **performance stress test**, not a realistic mission. Real TBD Reforger missions stay
well under **~10,000 slots** (Arma Reforger itself makes anything near that unrealistic), and the future
dynamic entities (vehicles, etc.) are expected to stay in the same order of magnitude. This bug lives
entirely above that ceiling, so it does not affect real usage and is recorded rather than fixed.

## Symptoms (observed @ ~518k)

1. **Ghost selection.** After select → deselect, some objects stay highlighted (yellow) while the
   toolbelt reads `SEL 0`. The store `selection.ids` is empty but the icon-cache `.selected` flags (or
   the rendered colors) have not cleared — the two have desynced.
2. **Can't select the ghosts.** Clicking a still-highlighted object does not select it.
3. **Copy/paste appears dead.** Ctrl+C / Ctrl+V do nothing. Almost certainly **downstream of #1**: the
   copy handler reads `selection.ids` (`MissionCreatorPage` `onKeyDown`, `KeyC` branch); when that is
   empty (the ghost state), the in-editor clipboard stays empty and Ctrl+V no-ops.

**Environment:** detail render mode (deck zoom ≈ −3.40; cluster mode only engages at zoom ≤
`ZOOM_CLUSTER_MAX = -4`). The 517k doc was built by **in-session copy/paste**, not a reload.

## Analysis (not fully root-caused — needs in-browser reproduction)

The entire selection / picking / render subsystem is **byte-identical before and after the T-145 flip**
(F3/F3.1 changed only the doc-core: `ydoc.ts` mutators, the hooks, undo, persistence, the barrel).
Untouched: `useSelectTool`, `slotIconCache`, `slotSpatialIndex`, `slotClusterIndex`, `useIconLayer` /
`useClusterIconLayer` / `useSelectionLayer`, the copy/paste handler, `setSelection`. So this is a
**pre-existing scale limit** the gate exposed, not a flip regression.

Caveat worth recording: the flip's F3.1 `okPatch` test proved the store **dictionaries** stay
byte-identical to the wasm doc, but it never asserted the **icon-cache / spatial-index / cluster-index**
state. A future investigation should not assume those caches are correct at scale just because the store
dicts are.

**Leading suspect — uncapped marquee selection.** The paste path deliberately caps its post-paste
selection at `BULK_SELECT_CAP = 500` (`MissionCreatorPage`), with the comment that putting ~10k ids in
`selection.ids` "blows up the highlight Set + outliner re-render." The **marquee release** in
`useSelectTool` (`onPointerUp`, marquee branch) has **no such cap** — a zoomed-out marquee over 500k
slots calls `slotSpatialIndex.pickRect(...)` and dumps the entire result into `selection.ids`. A
pathological six-figure selection stresses `setSelectionFlags` (O(n) over all icons), the virtual
outliner, and the deck color attribute — which fits the "worked at small scale, broke at large scale"
report. This is a hypothesis, not a confirmed cause.

## If/when this is revisited

Diagnose before touching correct-looking, unchanged code:

1. **Headless ~500k reproduction** (vitest, no GPU) that drives the real state pipeline — seed ~500k
   slots via `useMapStore._applySnapshot`, then `slotSpatialIndex.pickRect` → `setSelection` →
   `setSelectionFlags` → deselect, asserting (a) zero residual `.selected` after deselect, (b)
   `pickNearest` still returns the right id, (c) icon-cache dense ids == `Object.keys(slotsById)` ==
   spatial-index row set after a run of `pasteSlots`/`moveEntities`/`removeEntities`. This isolates a
   **state/index desync** (fixable headlessly) from a **Deck-render-only** refresh bug (needs a browser).
2. **Fix** per the evidence — most likely cap or otherwise bound the marquee selection (mirroring the
   paste cap), or make the selection machinery tolerate huge id sets.

**Critical files:** `apps/website/frontend/src/features/tactical-map/tools/useSelectTool.ts` ·
`state/slotIconCache.ts` · `state/slotSpatialIndex.ts` · `state/slotClusterIndex.ts` ·
`layers/useIconLayer.ts` · `mission-creator/MissionCreatorPage.tsx` · `state/useMapStore.ts`.

**Related:** T-059 (`BULK_SELECT_CAP`, bulk paste/delete) · T-065 (cluster / LOD @ extreme zoom) ·
T-067 (chunk cull, later reverted) · T-090 (map/scale program) · T-145 (Rust/wasm doc-core flip).
