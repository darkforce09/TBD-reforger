**Status:** frozen record

# T-090.4 — Z placement audit (buried / floating objects)

**Ticket:** T-090 · **Slice:** T-090.4  
**Status:** Spec ready (blocked on **T-090.3** export + **T-091** DEM)  
**Executor:** **claude-code**  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

**Phase A:** offline tool compares each exported map object's **pivot Z** against the **T-091 DEM** at `(x,y)` — fast pass over **1M+** Eden objects. **Phase B** (geometry-aware OBB sampling, buried/floating **visibility**) → **T-090.6**. This slice is Phase A only — **detect**, no auto-fix.

---

## Problem

Some Enfusion map objects export with pivots at ground, roof, or model center. Bridges span air; trees on slopes disagree with 16-bit DEM sampling. Without an audit, T-090.5 renders props at wrong heights and mission makers lose trust in the basemap.

**User requirement:** Notice the problem now; **solution comes later** (snap-to-DEM, pivot correction, or mod `GetSurfaceY` overlay).

**Scale note:** Eden has **~1M map objects** — manual verification impossible. T-090.4 is the **cheap full-catalog screen** (one DEM sample per object). False negatives on tilted/large props are expected; **T-090.6** runs **simplified 3D bounds math** (center + rotation + OBB corner samples) to detect what point audit misses — see [`t090_6_geometry_placement_audit.md`](t090_6_geometry_placement_audit.md).

---

## Algorithm

For each instance in catalog:

1. Sample `demZ = sampleDem(manifest.dem, x, y)` — same bilinear path as T-091 `elevationAt`.
2. If `z` missing: set `zSource = "missing"`, `demZ` only, `severity = warn`.
3. Else `zDeltaM = z - demZ`.
4. Compare to per-kind threshold:

| `kind` | `warn` (m) | `fail` (m) |
|--------|------------|------------|
| `road` | 0.3 | 0.5 |
| `building` | 0.5 | 1.5 |
| `tree` | 1.0 | 2.5 |
| `vegetation` | 0.8 | 2.0 |
| `prop` | 0.5 | 1.5 |
| default | 0.5 | 1.5 |

5. Special case: `tags` includes `bridge` or `elevated` → skip fail (warn only if \|zDelta\| > 5 m).

---

## Output: `z-audit.json`

```json
{
  "terrainId": "everon",
  "generatedAt": "2026-06-26T12:00:00Z",
  "demPath": "dem/everon-dem-16bit.png",
  "summary": {
    "total": 100000,
    "ok": 92000,
    "warn": 6000,
    "fail": 2000,
    "missingZ": 1500
  },
  "byKind": { "tree": { "fail": 800 }, "building": { "fail": 400 } },
  "failures": [
    {
      "id": "obj-123",
      "kind": "tree",
      "class": "conifer",
      "x": 5120,
      "y": 5120,
      "z": 142.0,
      "demZ": 138.2,
      "zDeltaM": 3.8,
      "severity": "fail",
      "resourceName": "{GUID}Prefabs/..."
    }
  ],
  "failuresTruncated": true,
  "maxFailuresListed": 5000
}
```

Full failure list may be separate `z-audit-failures.jsonl` for AI ingestion.

---

## Deliverables

| # | Path | Notes |
|---|------|-------|
| 1 | `scripts/map-assets/run-z-audit.ts` | Node; reads gz catalog + DEM |
| 2 | `packages/map-assets/everon/objects/z-audit.json` | Committed after first export |
| 3 | Vitest | Unit tests with synthetic DEM + 3 fixtures |

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| Z1 | Known buried fixture → `severity: fail`, zDeltaM < 0 | test |
| Z2 | Known floating fixture → `severity: fail`, zDeltaM > 0 | test |
| Z3 | Bridge tagged → not fail | test |
| Z4 | Summary counts match | test |
| Z5 | Run on sample golden → report generated | script |

---

## User-visible trust (T-090.9)

`severity` (`ok`/`warn`/`fail`) is surfaced to the mission maker as a **Z-trust badge** in the world
object inspect panel (GAP-M3) — `fail` reads "buried/floating — verify in Workbench". The audit detects;
the human-facing surfacing ships in [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md),
not "future".

## Future work (not this slice)

| Slice / ticket | Fix |
|----------------|-----|
| **T-090.6** | Geometry-aware audit — OBB samples, `visibleAboveGroundPct`, inter-object bounds |
| T-090.7 (**optional**) | Auto visual Z correction from geometry audit — optional enhancement, **not** deferred core; the Z-trust badge (T-090.9) is the shipped surfacing |
| T-092 | Spawn uses mod surface, not export Z |
| T-129 | Per-floor Z bands for buildings |

---

## Related

- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) §Z placement
- [`t090_6_geometry_placement_audit.md`](t090_6_geometry_placement_audit.md) — Phase B OBB / visibility audit @ 1M scale
