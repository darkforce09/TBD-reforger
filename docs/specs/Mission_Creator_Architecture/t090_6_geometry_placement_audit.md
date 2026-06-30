# T-090.6 — Geometry-aware placement audit (simplified 3D bounds)

**Ticket:** T-090 · **Slice:** T-090.6  
**Status:** Spec ready (blocked on **T-090.4** point audit + **T-090.3** bounds export)  
**Executor:** **claude-code** (+ optional **workbench** plugin for ground truth)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

For **every exported map object** (Eden-scale **1M+**), use **center + rotation + simplified 3D bounds** (not full meshes) to compute which parts are **above terrain**, **buried**, or **inside another object** — fully automated, no manual eyeballing.

**Building geometry (N6):** **Normative shipped geometry:** oriented bounding **rectangle** from
`spatial.halfExtentsM` + `rotationDeg`. Real **footprint polygon rings** are populated only when
T-090.3.0 proves Enfusion footprint export; when present, polygons supersede OBB rectangles for render.
The OBB audit below uses the same `halfExtentsM` + yaw.

**Axis remap (L2):** Enfusion local `up` maps to world Z; the bounds `localUp → world Z` remap +
rotation handedness are measured once by the **T-090.3.0** spike (S6/K6) and applied identically here and
in the glyph rotation ([`t090_world_object_glyphs.md`](t090_world_object_glyphs.md)). Document the
measured remap in the export script.

---

## Why T-090.4 alone is not enough

**T-090.4** samples DEM at a **single `(x,y)` pivot**. That is cheap (O(n) over the catalog, runs in Node) but wrong for:

| Case | Pivot looks OK | Geometry is wrong |
|------|----------------|-----------------|
| Tree on slope | Trunk base at DEM | Canopy pivot buried or floating |
| Tilted fence / wire | Center on line | One end underground |
| Large building | Corner pivot | Opposite corner floats |
| Bridge deck | Mid-span pivot | Legs not checked |
| Rock half-buried | **Intentional** | Should pass as `partial_burial_ok` |

**Product requirement (2026-06):** Cannot manually verify 1M Eden objects. Need **math on simplified geometry**, optionally confirmed by a **Workbench trace plugin** on a sampled subset.

---

## Strategy: two-phase audit pipeline

```text
Phase A — T-090.4 (point audit)     →  flag obvious outliers on ALL instances (~minutes)
Phase B — T-090.6 (geometry audit)  →  refine ALL instances OR only Phase A failures (~hours)
Phase C — workbench spot-check      →  0.1% random + all `fail` rows for calibration
```

**Default for 1M catalog:** Run Phase B on **every** instance if bounds are cheap (OBB only). Skip inter-object checks in Phase B unless `kind ∈ {building, prop}` and spatial neighbor count < 50.

---

## Core idea (user proposal — normative)

1. **Export per instance:** world position (center/pivot), **rotation** (yaw/pitch/roll or quaternion), **simplified bounds** (AABB or OBB extents in local space — from prefab metadata, not rendered mesh).
2. **Transform** local bound samples → world space.
3. **Compare** each sample against:
   - **Terrain:** DEM `sampleElevation(x,y)` and/or mod `GetSurfaceY(x,z)` (Workbench plugin).
   - **Other objects (optional):** simplified bounds of neighbors within radius R.
4. **Classify visibility / penetration:**
   - `visibleAboveGroundPct` — fraction of bottom-face samples with `z >= demZ - ε`
   - `maxBurialM` — max(`demZ - z_sample`) over samples below terrain
   - `maxFloatM` — max(`z_sample - demZ`) over samples above terrain
   - `intersectsOther` — bool + nearest id (buildings/props only)

No full 3D model required — **convex proxy** (box or 8–26 sample points) is enough for audit.

---

## Simplified geometry models (pick one per prefab)

| Level | Data | Sample count | Accuracy | Export cost |
|-------|------|--------------|----------|-------------|
| **L0** | Pivot only | 1 | Low | T-090.4 |
| **L1** | AABB (axis-aligned, ignores yaw) | 8 corners | Medium | Cheap |
| **L2** | **OBB** (extents + yaw) | 8 corners + 6 face centers | **Recommended default** | Prefab bbox from Workbench |
| **L3** | OBB + vertical extrusion (heightM) | 26 points | High for buildings/trees | Prefab + kind heuristic |
| **L4** | Convex hull (≤12 verts) | 12–24 | Best offline | Heavy export; defer |

**Schema fields** (add to T-090.2 catalog row — optional but expected after T-090.3):

```json
{
  "bounds": {
    "model": "obb",
    "pivot": "center",
    "halfExtentsM": { "x": 2.5, "y": 1.0, "z": 4.0 },
    "rotationDeg": { "yaw": 45, "pitch": 0, "roll": 0 }
  }
}
```

Notes:

- Editor/store **y = north**; bounds local **z** often = vertical in Enfusion — document axis remap in export script (`localUp → world Z`).
- If bounds missing → fall back to **kind defaults** (see §Kind defaults).

---

## Algorithm (Phase B — offline Node or worker)

For each instance `obj`:

### Step 1 — Build sample points

```text
samples = transformOBB(obj.bounds, obj.x, obj.y, obj.z, obj.rotationDeg)
// Minimum: 8 corners. Recommended: 8 corners + bottom face 4 edge midpoints (12 total).
```

### Step 2 — Terrain penetration

For each sample `(sx, sy, sz)`:

```text
demZ = sampleDem(dem, sx, sy)
delta = demZ - sz   // positive => sample is BELOW terrain surface (buried)
```

Aggregate:

```text
buriedCount = count(delta > BURIAL_EPS_M)
floatCount  = count(-delta > FLOAT_EPS_M)  // sample above terrain
visibleAboveGroundPct = 1 - buriedCount / samples.length
maxBurialM = max(delta where delta > 0)
maxFloatM  = max(-delta where delta < 0)
```

### Step 3 — “Comes out of the ground” (tilted objects)

For objects with **pitch/roll ≠ 0** or **large horizontal extent**:

- Compute **lowest sample** `zMin` and **highest sample** `zMax`.
- Compute `terrainBand = max(demZ at all sample XY) - min(demZ at all sample XY)`.
- **Pass partial burial** if:
  - `zMax - zMin > terrainBand + 0.5m` AND
  - `visibleAboveGroundPct >= 0.25` AND
  - `maxBurialM < kindThreshold` (rock, ruin — allow deep burial)

This matches “find where it comes out” without mesh CSG.

### Step 4 — Inter-object penetration (optional, expensive)

Only when `obj.kind ∈ {building, prop, rock}`:

```text
neighbors = spatialHash.query(obj.x, obj.y, radius = max(halfExtent) * 2)
for n in neighbors:
  if obbIntersects(obj.bounds, n.bounds): flag intersectsOther
```

**1M scale:** spatial hash grid **64 m cells**; expect O(k) per object, k ≪ n. Run inter-object only on Phase A `warn|fail` rows if full pass too slow (>30 min).

### Step 5 — Severity

| Condition | Severity |
|-----------|----------|
| `visibleAboveGroundPct >= 0.9` AND `maxFloatM < floatWarn` | `ok` |
| Partial burial allowed (rock, ruin, tagged `partial_burial_ok`) | `ok` or `warn` |
| `visibleAboveGroundPct < 0.5` AND `maxBurialM > burialFail` | `fail` buried |
| `visibleAboveGroundPct >= 0.9` AND `maxFloatM > floatFail` | `fail` floating |
| `intersectsOther` AND not tagged `stacked` | `warn` |
| bounds missing, L0 only | inherit T-090.4 |

---

## Kind defaults (when prefab has no bounds)

| `kind` | Default halfExtentsM (x,y,z) | pivot |
|--------|------------------------------|-------|
| `tree` | 1, 1, 6 | base |
| `building` | 8, 8, 4 | center |
| `road` | — (use line geometry, sample every 4 m) | — |
| `prop` | 0.5, 0.5, 1 | base |
| `rock` | 2, 2, 1.5 | center |

Roads: sample **polyline vertices** + midpoint every **4 m** against DEM (not OBB).

---

## Workbench plugin path (ground truth — recommended for calibration)

Offline DEM is ~2 m grid; Enfusion **`GetSurfaceY`** is authoritative for Reforger.

**Plugin:** `TBD_MapObjectAuditPlugin.c` (new, T-090.3 or T-090.6)

For each entity (or batch from catalog id list):

1. Read world transform + bounding box from prefab (`GetBounds()` or equivalent Enfusion API).
2. For each corner: `surfaceY = GetSurfaceY(x, z)`.
3. Compare entity vertex world Y vs `surfaceY`.
4. Optional: **downward trace** (`TraceMove` / line trace) to detect if point is **inside** another entity’s collision geometry.

Export append-only **`z-audit-workbench.jsonl`** for regression — Node audit must agree within **0.5 m** on calibration set.

**AI note:** Prefer plugin traces for **golden calibration** (100 objects); Node OBB audit for **full 1M pass**.

---

## Output: extend `z-audit.json`

```json
{
  "auditVersion": 2,
  "geometryModel": "obb-12sample",
  "summary": {
    "total": 1048576,
    "ok": 980000,
    "warn": 52000,
    "fail": 16576,
    "boundsMissing": 120000
  },
  "failures": [
    {
      "id": "obj-991",
      "kind": "building",
      "severity": "fail",
      "reason": "buried",
      "visibleAboveGroundPct": 0.12,
      "maxBurialM": 2.3,
      "maxFloatM": 0.1,
      "sampleCount": 12,
      "boundsModel": "obb",
      "pointAuditZDeltaM": 0.2
    }
  ]
}
```

Separate **`z-audit-failures.jsonl`** for AI — one JSON object per line, all `fail` rows.

---

## Performance budget (1M objects)

| Stage | Target | Technique |
|-------|--------|-----------|
| Load catalog | <60 s | Stream gzip JSONL; don't hold full array |
| OBB + DEM samples | <20 min | Worker pool; 12 samples × 1M = 12M DEM lookups (cheap) |
| Inter-object | <30 min optional | Spatial hash; only warn/fail from point audit |
| Report write | <30 s | Streaming jsonl |

If >45 min total, run inter-object on subset only.

---

## Deliverables

| # | Path |
|---|------|
| 1 | `scripts/map-assets/run-geometry-audit.ts` |
| 2 | `scripts/map-assets/obbSamples.ts` — transform + sample helpers |
| 3 | `scripts/map-assets/spatialHash.ts` — neighbor query |
| 4 | Optional `apps/mod/.../TBD_MapObjectAuditPlugin.c` |
| 5 | Vitest fixtures: tilted box, half-buried rock, floating building |
| 6 | Extend T-090.2 schema `bounds` block |

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| G1 | Tilted OBB: 3/12 samples buried, `visibleAboveGroundPct ≈ 0.75` → `ok` | test |
| G2 | Fully buried box → `fail` | test |
| G3 | Floating box (all samples +2 m) → `fail` | test |
| G4 | Rock with `partial_burial_ok` tag → not `fail` | test |
| G5 | 100-object Workbench calibration vs Node within 0.5 m | manual once |
| G6 | Full Everon run completes under perf budget | ops log |

---

## Relationship to render (T-090.5)

- Render uses export `z` when `geometryAudit.severity !== fail`.
- On `fail`: render at **corrected** position optional (T-090.7) — shift so lowest sample sits on DEM (visual only).

---

## Out of scope

- Full mesh CSG / boolean operations
- Real-time editor “select object → highlight buried verts” (future UX)
- Building floor slicing (**T-126**)

---

## Related

- [`t090_4_z_placement_audit.md`](t090_4_z_placement_audit.md) — Phase A point audit
- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) — `bounds` field
- [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md) — export bounds from Workbench
- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — DEM + GetSurfaceY plugin pattern
