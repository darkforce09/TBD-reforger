# T-090.1.1.1 — Claude Code handoff (Map land-cover compose)

**Slice:** T-090.1.1.1 · **Executor:** claude-code · **Branch:** `main` (single lane — no worktree)  
**Spec (authority):** [`docs/specs/Mission_Creator_Architecture/t090_1_1_1_map_landcover_compose.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_1_1_map_landcover_compose.md)  
**Program hub:** [`t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)

---

## What you are building

Break the **uniform green** Map tab into readable **land-cover tints** (forest darker, open fields lighter/tan) by extending the offline compose script — then rebuild **`tiles/map/`** only.

```text
P0 spike (L1/L2) → extend build-map-cartographic.mjs
  → make map-cartographic-everon
  → verify M1–M4, M7, M8 → tag T-090.1.1.1
```

**No frontend changes expected** unless manifest fields need a label tweak — Map radio + pyramid loader already shipped @ T-090.1.1.

---

## Bootstrap already on `main` (do not redo)

| Area | State |
|------|-------|
| `build-map-cartographic.mjs` | G1-A upscale + `.2.5.2` water + `.topo` roads — **extend here** |
| Map pyramid + UI | Shipped @ `6e06e679` — `useTerrainBasemapLayer` map branch live |
| Satellite | Unified bundle @ T-090.1.2.8 — **frozen** |
| Everon DEM | `packages/map-assets/everon/dem/` — read-only for L2 hillshade/slope |
| SAP ortho (L1 input) | Staging under `packages/map-assets/everon/staging/sap/` (local/gitignored) — use for RGB heuristic, **do not** write back to satellite bundle |

**Baseline:** `./scripts/ticket brief T-090 --slice T-090.1.1.1` · `make schema-validate` exit 0.

---

## P0 spike (do first)

Write **`.ai/artifacts/t090_1_1_1_source_spike.json`**:

```json
{
  "winner": "L1" | "L1+L2" | "...",
  "rejects": [{ "id": "L3", "reason": "..." }],
  "heuristic": { "forest": "...", "open": "...", "urban": "..." },
  "notes": "..."
}
```

| ID | Approach | When to use |
|----|----------|-------------|
| **L1** | Classify SAP RGB → forest / open / urban-ish buckets; paint before roads/water | **Default ship path** |
| **L2** | DEM slope + elevation bands (subtle relief multiply) | Combine with L1 |
| **L3** | Forest region polygons | Only if partial export exists — else honest-stop |
| **L4** | Engine land-cover export | Same dead-end as T-090.1.2.4 — document reject |

**Pass bar:** @ default MC zoom, operator can point at a forest patch vs adjacent field and see **two distinct tints** (M3).

---

## Execution order

1. **P0** — spike JSON + chosen heuristic documented.
2. **P1** — land-cover pass in `build-map-cartographic.mjs` (after TGA load / during upscale pipeline — your call; log in spike).
3. **P2** — `make map-cartographic-everon` (rebuilds staging ortho + pyramid + manifest).
4. **P3** — `.ai/artifacts/t090_1_1_1_verify_log.md` with M1–M4, M7, M8 output + M3 screenshot path/coords.
5. Tag **`T-090.1.1.1`** · prefix **`T-090.1.1.1:`**

---

## Do not

| Forbidden | Why |
|-----------|-----|
| Edit `docs/**`, registry, CLAUDE status | Cursor doc sync after ship |
| Touch Satellite ortho / `everon-sat.tbd-sat` / `make map-water-everon` | Separate product tab |
| Wait for T-090.3 object export | Land-cover LUT is heuristic until regions ship |
| Add Deck.gl object layers | T-090.5 |

---

## Key files

| Path | Role |
|------|------|
| `scripts/map-assets/build-map-cartographic.mjs` | **Primary edit** |
| `scripts/map-assets/decode-topo.mjs` | Road vectors (read-only) |
| `packages/map-assets/everon/manifest.json` | Patched by make target |
| `Makefile` | `map-cartographic-everon`, `map-cartographic-verify` |

---

## After ship

Cursor doc sync · next slice **T-090.1.2.9** (Satellite road overlay) · then **T-090.3** export pipeline.
