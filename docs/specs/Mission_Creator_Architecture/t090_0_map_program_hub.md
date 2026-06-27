# T-090.0 — Map program hub + manifest schema

**Ticket:** T-090 · **Slice:** T-090.0  
**Status:** **active** (cursor-docs)  
**Executor:** cursor-docs  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) (program hub)

---

## In one sentence

Publish the map/terrain program hub, `terrain-manifest.schema.json`, stub Everon manifest, Git LFS rules, and verification script **specs** so T-091.0 export and T-090.1 code have a frozen contract.

---

## Problem

Editor uses a procedural grid only; no manifest, no aligned tiles, no CI gate tying `terrains.ts` to exported assets. Prior docs buried export steps in a single thin hub with no per-slice verification.

---

## Goal

1. Program hub [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) — order, coordinates, asset layout, links to **every slice spec**.
2. Per-slice specs: **T-090.1**, **T-091.0–.2**, **T-092.0–.2** (this slice creates the doc set).
3. [`terrain-manifest.schema.json`](../../../packages/tbd-schema/schema/terrain-manifest.schema.json) + stub [`manifest.json`](../../../packages/map-assets/everon/manifest.json).
4. [`terrain-anchors.schema.json`](../../../packages/tbd-schema/schema/terrain-anchors.schema.json) + example [`verification.example.json`](../../../packages/map-assets/everon/anchors/verification.example.json).
5. Root [`.gitattributes`](../../../.gitattributes) — LFS for `packages/map-assets/**/*.png` and `*.webp`.
6. Verify scripts: [`verify-terrain-manifest.ts`](../../../scripts/website/verify-terrain-manifest.ts), [`verify-terrain-alignment.ts`](../../../scripts/website/verify-terrain-alignment.ts).
7. DEV_RUNBOOK §Map assets + `packages/map-assets/README.md`.
8. Fix [`terrains.ts`](../../../apps/website/frontend/src/features/tactical-map/coords/terrains.ts) — Biki bounds/heights, `manifestUrl`, `heightRangeMinM`/`heightRangeMaxM`.

---

## Out of scope

- Tile rendering code (**T-090.1**)
- DEM PNG in repo (**T-091.0** human)
- Compiler/mod spawn (**T-092**)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Program order | **T-090.0 → T-091.0 → T-090.1 + T-091.1–.2 → T-092 → T-071 → T-068.13 → T-068.7+** |
| Stub manifest | `widthPx`/`heightPx`: **0** until export; verify scripts **warn + pass** in stub mode |
| DEM pixel size | **Record at export** — do not hard-code 6400² (Biki ~2 m → *expected* ~6400 if full grid) |
| Horizontal coords | Deck Cartesian meters; **no Web Mercator** |
| Spawn authority | Engine `GetSurfaceY` + capsule offset (**T-092**) — not DEM alone |
| Storage precision | **0.001 m** display/storage in editor; DEM native ~2 m |

---

## Deliverables checklist

| # | Artifact | Path |
|---|----------|------|
| D1 | Program hub | `t090_091_map_terrain_program.md` |
| D2 | Slice specs | `t090_1_*`, `t091_0_*`, `t091_1_*`, `t091_2_*`, `t092_0_*`, `t092_1_*`, `t092_2_*` |
| D3 | Manifest schema | `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| D4 | Anchors schema | `packages/tbd-schema/schema/terrain-anchors.schema.json` |
| D5 | Stub manifest | `packages/map-assets/everon/manifest.json` |
| D6 | Anchor example | `packages/map-assets/everon/anchors/verification.example.json` |
| D7 | Git LFS | `.gitattributes` |
| D8 | Verify scripts | `scripts/website/verify-terrain-*.ts` |
| D9 | Registry + sync | `.ai/tickets/registry.json` → `./scripts/ticket sync` |

---

## Verification gate (mandatory)

**Advance T-090.0 → T-091.0 only when ALL PASS.**

### Automated (exit 0)

```bash
cd /home/Samuel/Projects/TBD-Reforger

# Registry + docs
./scripts/ticket sync
./scripts/ticket check
make ticket-check-strict

# Schema + stub manifest
make schema-validate

# Terrain cross-check (stub mode OK)
make verify-terrain

# Frontend still builds
cd apps/website/frontend && npm run build && npm run lint
```

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | Hub links | Hub §Slice index links to **all eight** slice spec files |
| S2 | Manifest schema | Stub `manifest.json` validates (stub dims 0 allowed) |
| S3 | Anchors schema | `verification.example.json` validates |
| S4 | terrains.ts | Everon 12800², Arland **4096²**; height ranges match Biki |
| S5 | terrains ↔ manifest | `verify-terrain-manifest.ts` exit 0 |
| S6 | Alignment stub | `verify-terrain-alignment.ts` exit 0 with stub warning |
| S7 | ACTIVE NOW | `CLAUDE.md` sync block: **T-090 — T-090.0** |
| S8 | No legacy IDs | `make ticket-check-strict` exit 0 |
| S9 | LFS | `.gitattributes` covers map-assets PNG/WebP |

### Human (optional @ T-090.0)

| ID | Check | Pass condition |
|----|-------|----------------|
| H1 | Read hub | Operator can follow T-091.0 runbook without opening plan file |

---

## Advance slice

When all **S1–S9** pass:

```bash
./scripts/ticket advance-slice T-090   # → T-091.0 (human export) or T-090.1 after tiles exist
```

Per program order, **T-091.0** (human DEM + tiles + anchors) runs before **T-090.1** code needs tile files.

---

## Related

- [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md)
- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md)
- [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)
