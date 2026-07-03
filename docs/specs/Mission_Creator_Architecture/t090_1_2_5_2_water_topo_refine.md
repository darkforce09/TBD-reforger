# T-090.1.2.5.2 — Exact road geometry water refine + one-button pipeline

**Ticket:** T-090 · **Slice:** T-090.1.2.5.2  
**Status:** **SHIPPED** @ `1c07d97a` (tag **T-090.1.2.5.2**) — operator: **good enough** (2026-07-03)  
**Executor:** claude-code  
**Depends on:** **T-090.1.2.5.1** @ `82488c6f`  
**Supersedes for:** residual road FP @ operator viewport (~4617, 8711); `.topo` road-corridor guard  
**Far-future perfection:** **T-143** (`idea`) — exact hydrology + MC water placement guard  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Decode **`Eden.topo` offline**, use the engine road network as a **deterministic subtraction guard** (`roadFrac`), relax wet-channel rules for valley gullies, and ship **`make map-water-everon`** as the one-button rebuild — accepting that **no hydro layer exists in `.topo`** (G1-B).

---

## Spike verdict (G1-B)

| Finding | Detail |
|---------|--------|
| **`.topo` decoded** | `scripts/map-assets/decode-topo.mjs` — 6 LOD sections × 888 records; `[u8 type][u32 n][n×f32LE pairs][u32 K][K attrs]`; y = north-up image metres |
| **Classes** | All five types = **road/airfield** network; trailing `PWLN` = powerlines |
| **No hydro in `.topo`** | Water vectors not in this file → road subtraction + relaxed classifier, not vector water mask |
| **Honesty** | Several long `.2.5/.2.5.1` "rivers" were **roads** (uniform width, junctions, asphalt) — now correctly grey |

Artifacts: [`.ai/artifacts/t090_1_2_5_2_source_spike.json`](../../../.ai/artifacts/t090_1_2_5_2_source_spike.json) · [verify log](../../../.ai/artifacts/t090_1_2_5_2_verify_log.md)

---

## Mask recipe (shipped)

| Layer | Mechanism |
|-------|-----------|
| Ocean | DEM ≤ 0 (unchanged) |
| Compact | `.2.5.1` compact rules (flatFrac ≤ 0.12) |
| Grey-river linear | `.2.5.1` + **`roadFrac ≤ 0.45`** guard vs `.topo` corridors |
| Wet-channel | Relaxed: valleyFrac ≥ 0.6, meanSat ≤ 0.18, meanLum ≤ 0.31, area ≥ 1000 m² + roadFrac guard |
| Road subtract | `.topo` half-widths px@4 m by type (0=3, 1=2, 2=2, 3=1, 5=1) |

**153 accepted bodies** — operator FP viewport **zero** water bodies; 123 wet-channel gullies blue.

---

## One-button pipeline

```bash
make map-water-everon
```

Restore → mask → composite → unified bundle → manifest bytes patch → lossless pyramid → verifies. Terrain config: `TOPO_TERRAINS` in `decode-topo.mjs` (Arland row seeded).

**Not** full `make map-export` — water-only step toward T-090.3 automation.

---

## Operator acceptance (2026-07-03)

**Verdict:** Good enough for now. Important hydrology (ocean, lake, gullies) reads; minor misses acceptable — missions playtested. Named rivers only in BI map / `Eden.ent` → **T-143** / **T-090.8**.

| ID | Result |
|----|--------|
| R-FP viewport ~(4617, 8711) | PASS — crop committed |
| R-FN gullies | PASS — west + SE crops |
| R-REG lake / ocean / land | PASS |

---

## Escalation (deferred — T-143)

Exact pixel-perfect hydrology requires **`Eden.ent` water entities** (pak codec locked) or Workbench export — see spike JSON + **T-143**. MC **placement guard** (block units in ocean / large water) also **T-143**.

---

## Related

- Parent: [`t090_1_2_5_1_water_mask_refine.md`](t090_1_2_5_1_water_mask_refine.md)  
- [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md) — full export north star  
- **T-143** — perfect water (`idea`)
