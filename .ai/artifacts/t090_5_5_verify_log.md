# T-090.5.5 — Verify log: Map Engine v2 tree / veg / prop glyphs (IconLayer)

**Slice:** T-090.5.5 · **Executor:** claude-code · **Date:** 2026-07-05
**Authority:** plan §7 row T-090.5.5 · [`t090_5_map_object_render_layer.md`](../../docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md) · [`t090_world_object_glyphs.md`](../../docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md) · [`t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md) (N2/N3/LOD3/LOD5)

---

## What shipped

The last Map Engine v2 render lane: **individual tree/prop glyphs** drawn as Deck `IconLayer`s
(`world-trees` slot 9, `world-props` slot 10) over the existing world-glyph atlas. The 501,861
Everon trees the worker already indexed (T-090.5.3) now render at `deckZoom ≥ TREE_GLYPH_MIN_ZOOM (0)`;
below the band they stay hidden and the forest-mass polygons (T-090.8.1) carry readability. **No
world supercluster** (contract LOD5).

### Files

| File | Change |
|------|--------|
| `workers/worldObjectsCore.ts` | `WorldPrefabRow` + `narrowPrefabRows` carry `render{iconKey,baseSizePx,defaultColor,importanceZoom}` + `spatial.heightM`; **`visibleInstances` self-hydrates** the covering chunks (skip-when-invisible guard) so it's a standalone tree/prop driver — INSTANCE_BUDGET cap unchanged |
| `worldmap/treeStore.ts` (new) | viewport streaming singleton (mirror forestMassStore): manifest→glyph lookup, `setTreeViewport` band+toggle gate + chunk-set/band dedupe, `worldVisibleInstances`→partition into tree/prop `TreeGlyphComposite`s, stale-reply supersede |
| `worldmap/treePropLayer.ts` (new) | pure helpers (`deckAngleForRotationDeg`, `treeSizeMultiplier`, `glyphSizeMeters`, `hexToRgba`) + `buildTreeGlyphLayer`/`buildPropGlyphLayer` (IconLayer over binary attrs + `getIcon` accessor, `sizeUnits:'meters'`, `pickable:false`, null-degrade) |
| `worldmap/useWorldMapLayers.ts` | `ensureTreeStream` per terrain; `setTreeViewport` on viewport (toggle/band-gated in store); `useSyncExternalStore` tree+prop composites; `treesVisible`/`propsVisible` gates; `appendGlyphLayers` after forest |
| `worldmap/treePropLayer.test.ts` (new) | pure helpers (R8/GL-G5 rotation, size cap, tint) + layer builders + treeStore (LOD3, band, partition, dedupe, R11, supersede) |
| `workers/worldObjectsCore.test.ts` | render/heightM passthrough + self-hydration + skip-when-invisible |

Glyphs: **no new SVGs** — the committed Everon census is trees only (conifer 295,126 /
deciduous 206,735; veg/rock/prop/utility = 0), and `tree-conifer`/`tree-deciduous` already have
SVG + manifest + atlas rects. The prop layer is real code exercised by a synthetic fixture.

---

## Automated gates — ALL PASS (exit 0)

| Gate | Result |
|------|--------|
| `make schema-validate` | **PASS** — validate.mjs + map-object enums/golden + `verify-map-glyphs` (28 glyphs) + type-inventory + `verify-t090-specs` (12 gates) + n6 + n10 |
| `make map-glyphs-verify` | **PASS** — 28 glyphs, golden + everon iconKeys covered, atlas rects verified |
| `npm run test -- --run treeProp lodGates worldObjectsCore` | **PASS** — 56/56 (3 files) |
| `npm run test -- --run` (full regression) | **PASS** — **240/240** (24 files; was 223 @ T-090.5.4, +17 new) |
| `npm run build` | **PASS** — tsc + vite clean (pre-existing MissionCreatorPage chunk-size warning only) |
| `npm run lint` | **PASS** — eslint clean |

### Key test coverage

- **R8 / GL-G5 rotation distinctness:** `deckAngleForRotationDeg(0) ≠ (90)` (0 vs −90), never −0.
- **LOD3 (contract):** treeStore @ −2 → no worker call, empty composites (trees hidden); @ 0 →
  tree composite populated, buildings dropped, props still gated out; @ +3 → props/rocks ride the
  prop group. Mirrors the worldObjectsCore W4 gate (`visibleInstances` empty @ −2/−3, trees @ 0).
- **INSTANCE_BUDGET:** worker cap asserted (`instanceBudget:4` → count 4); census invariant
  (501k trees > 150k budget, never streamed below band) already in worldObjectsCore.test.
- **Self-hydration:** `visibleInstances` returns trees after only `loadManifest` (no prior
  `loadChunksInBbox`).
- **Pan-stability / correctness:** chunk-set+band dedupe (repeat viewport = 1 fetch, band change =
  2); stale-reply supersede (exactly one commit); R11 empty-terrain path.

---

## Manual (operator — `VITE_WORLDMAP_ENABLED=1 make web`, hard refresh) — PENDING

GPU/browser checks (not runnable headless; consistent with prior render slices' operator pass):

- **LOD3:** @ −2 no tree icons (forest polygons only); zoom to 0+ → glyphs appear; forest fill
  fades per the α gate.
- **R5:** ≥55 fps panning a tree-visible viewport at zoom ≥ 0 (`FpsCounter`).
- **Toggle:** trees/props off (Mission Settings) → layers hidden.
- **M-reg:** flag OFF (default) → first paint identical to today (no fetch/worker/layers — the
  WORLDMAP_ENABLED-gated store bindings early-return; verified by the `build` + memo-off path).

---

## Notes for Cursor doc sync

1. **`render.importanceZoom` is carried but inert** for the current Everon census — no tree prefab
   declares it and the tree class gate is already 0, so per-prefab landmark early-surfacing has no
   data to act on. Class-gate visibility (contract N2/N3) is fully respected. The plumbing is in
   place (`WorldGlyphRender.importanceZoom` + `visibleWithImportance` in lodGates) for a future
   terrain/landmark set; no render-time application added this slice.
2. **Glyph baseSizePx divergence (latent, not a regression):** the glyphs-spec N4 table lists
   conifer 18 / deciduous 16, but the committed Everon prefab export sets `render.baseSizePx = 24`
   for trees. The layer uses the **per-prefab export value** (data-driven, correct). Flagged for
   reconciliation of the illustrative N4 table vs the export.
3. **Data flow:** trees stream via `worldVisibleInstances` (LOCKED), independent of the building
   `chunkStore`; `visibleInstances` was made self-hydrating so it needs no external chunk-load
   driver. Both stores share the one worker core.

**Ready for Cursor doc sync.**

---

## Fix follow-up (post-ship, same slice)

Operator reported **no tree glyphs render** even at zooms where the gate is open (buildings
~30 px ⇒ deckZoom ≈ +1). **Root cause:** the `world-trees`/`world-props` `IconLayer`s fed Deck a
binary `data: {length, attributes}` payload. IconLayer builds its per-instance `instanceIconFrames`
by iterating `data` through `getIcon`; the non-iterable binary form packs **zero icons** so nothing
draws. `PolygonLayer`/`SolidPolygonLayer` (buildings/forest) accept binary — IconLayer does not.

**Fix:** switched the glyph layers to the **object-array** `data` + per-datum accessors form that
`layers/useIconLayer.ts` uses for the 367k slot markers (`getIcon: d => d.iconKey`, `getPosition`,
`getAngle`, `getSize`, `getColor`). `TreeGlyphComposite` (SoA) → `TreeGlyphInstance[]` / `TreeGlyphSet`;
`treeStore.partition` builds object arrays; `treePropLayer` builders + tests updated. Pure helpers
and the worker (SoA `VisibleSet`, self-hydration) unchanged.

**Debug HUD (operator ask):** the DEV FPS badge (`mission-creator/FpsCounter.tsx`) now also shows
the live Deck zoom and the world-glyph draw count — `z <zoom> · glyph <n> · <fps> FPS`. Zoom via a
new transient `useMapStore.deckZoom` slice (mirrors `cursor`), set from TacticalMap's viewState
effect (no per-pan page render, T-057); count via `subscribeTreeStream` snapshots. Self-diagnosing:
count > 0 with nothing on screen ⇒ atlas/render; count 0 ⇒ stream/gate.

Re-verified: vitest **240/240**, `treeProp/lodGates/worldObjectsCore` **56/56**, build + lint green.
Operator browser confirm pending (zoom to ≥ 0 → green rotated tree glyphs over the forest).
