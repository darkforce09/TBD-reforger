# T-090.9 — World-object interaction (hover, inspect, filter, legend)

**Ticket:** T-090 · **Slice:** T-090.9
**Status:** Spec ready (depends on **T-090.5** render + **T-090.7** resolver)
**Executor:** **claude-code**
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · picking [`t090_world_objects_worker.md`](t090_world_objects_worker.md) · N7

---

## In one sentence

Static world objects become **read-only context you can interrogate** — hover for a tooltip, click for a
read-only inspect panel with "Ask AI about this object", filter/search by taxonomy, a legend, and a
Z-trust badge — without ever moving them (that stays in Workbench) and without re-enabling Deck GPU pick.

---

## Scope (N7 — locked)

| In T-090.9 | Forbidden in Mission Creator |
|------------|------------------------------|
| Hover tooltip | Move / delete / edit terrain props (**Workbench only**) |
| Click → read-only inspect panel | Deck GPU pick on world layers |
| Filter / search by taxonomy | Re-introducing Deck's `onHover` (removed in T-057) |
| Legend panel | Marquee-selecting world objects |
| Z-trust badge + "Ask AI about this object" | — |

World objects are **read-only context**. Editing terrain geometry is out of Mission Creator scope by
design — there is no "future" mutation path here.

---

## Picking architecture (N4/N7)

- A **separate world** rbush (`worldSpatialIndex`) in [`worldObjects.worker.ts`](t090_world_objects_worker.md)
  answers hover/click — the slot `slotSpatialIndex` singleton is untouched.
- **Hover:** the existing container `pointermove` (already feeding the toolbelt cursor, rAF-throttled)
  calls `pickNearest`. Deck's `onHover` was **removed** in T-057 and is not reintroduced; `getCursor`
  stays constant so Deck never runs a GPU hover pass.
- **Click precedence:** `slotSpatialIndex.pickNearest` wins (slots are the authored layer); if no slot is
  under the cursor, world inspect opens. **Alt+click** forces world inspect over a slot.
- **Marquee** stays slots-only.
- **Hover precedence:** slot tooltip if a slot is under the cursor, else the world tooltip.

This preserves the T-057 (no per-move GPU pick) and T-063 (rbush pick) performance contracts.

---

## Hover tooltip (fields)

| Object | Tooltip |
|--------|---------|
| building/tree/rock/prop | `label` · `class` · `heightM` (or `footprintM2`) · `cover.type` |
| road | `roadClass` · width |
| **forest region** | `"Mixed {dominantSpeciesClass} forest · ~{treeCount} trees · {areaHa} ha · {coverType} cover"` |

Throttled to one pick per animation frame; off-map → cleared.

## Inspect panel (read-only)

Renders [`ResolvedWorldObject`](../../../packages/tbd-schema/schema/map-object-resolved.schema.json):
identity (`label`, `class`, `taxonomyPath`, `summary`), placement (x/y/z, rotation), spatial (size,
footprint), gameplay (cover/LOS/movement/flags), and a **Z-trust badge** from `placement.severity`
(GAP-M3): `ok` (green) · `warn` (amber) · `fail` "buried/floating — verify in Workbench" (red). A
**"Ask AI about this object"** action sends `getAiContextPack([id])` to the editor AI. No edit controls.

## Filter / search (one shared `WorldObjectFilter` — GAP-H7)

The human filter panel and the AI read API ([`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md))
use the **same** `WorldObjectFilter` object — one source:

```ts
type WorldObjectFilter = {
  kinds?: WorldObjectKind[];
  classes?: string[];           // e.g. buildingClass=['military']
  coverType?: 'none' | 'soft' | 'hard';
  blocksInfantry?: boolean;
  maxPlacementSeverity?: 'ok' | 'warn' | 'fail';
  bbox?: Bbox;                  // "within current selection"
};
```

UI: kind checkboxes → class sub-checkboxes (e.g. "military buildings only"), a prefab text search
(`resourceName`/`label` substring), and a "within current selection bbox" toggle. Counts come from
`type-inventory.json` (L3) — e.g. "Buildings · military (12)".

## Legend (GAP-H6, L4)

A collapsible legend generated from the glyph manifest + enums: glyph swatch per class, `roadClass`
color/width key, forest fill swatch, and the Z-trust badge key. **Accessibility (L4):** encoding is never
color-only — every road class and glyph also carries a **shape/label**, so colorblind users can
distinguish them. Default-collapsed.

## Empty / "export not run" state (GAP-M7, L5)

When a terrain has no `objects/` export (e.g. **Arland** today), the world layers, filter and legend show
a first-run state: "World objects not exported for {terrain} yet — run `make map-export TERRAIN={id}`."
The basemap/grid still work; this is distinct from a tile 404 toast.

## Persistence (N8)

World-layer toggles persist in `localStorage` (`tbd-mc-world-layers`); the basemap view in
`tbd-mc-basemap-view`. Grid/hillshade remain per-mission `meta.environment` (see
[`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) §Persistence).

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| I1 | Hover a building → tooltip shows class + cover; no slot selection change | manual + vitest |
| I2 | Click a world object → inspect opens read-only; slot selection unchanged | manual |
| I3 | Alt+click over a slot → world inspect (slot otherwise wins) | manual |
| I4 | Filter "military buildings" → only military shown; count matches `type-inventory.json` | vitest |
| I5 | Z-trust badge reflects `placement.severity` | vitest |
| I6 | Legend distinguishes road classes by shape+label, not color alone | manual a11y |
| I7 | Arland (no export) → "not exported yet" state, editor still usable | manual |
| I8 | Pan ≥55 fps hovering over 50k visible world instances | FpsCounter |

---

## Out of scope
- World object mutation (Workbench only) · building floor picker (**T-129**).

## Related
- [`t090_world_objects_worker.md`](t090_world_objects_worker.md) · [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md)
- [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md) · [`t090_4_z_placement_audit.md`](t090_4_z_placement_audit.md)
