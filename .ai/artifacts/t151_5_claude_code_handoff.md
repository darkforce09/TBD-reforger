# T-151.5 — Claude Code handoff (glyph atlas: trees, props, badges)

**Spec (wins on conflict):**
[`t151_5_glyph_atlas.md`](../../docs/specs/Mission_Creator_Architecture/t151_5_glyph_atlas.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `552e68aa` (tag **T-151.4.1**) or later —
**never `main`**. Do **not** run `./scripts/ticket run`. Linear commits only.

## Operator report

T-151.4 + T-151.4.1 shipped: full vector stack on wgpu; buildings restored; road joins fixed.
**Forest mass still over-dense** (mega land-cover hull + TBDD iso=1) — operator wants **tree
glyphs first** so overdraw can be judged against real instances. Do **not** retune forest export
in this slice.

## CURRENT STATE (wgpu @ T-151.4.1)

| Layer | Status |
|-------|--------|
| Basemap / hillshade / grid | Yes |
| Sea / landcover / contours / roads / forest mass / buildings | Yes (W4) |
| Tree / prop / vegetation **glyphs** | **No** — W5 |
| Building badges | **No** — W5 |
| Slot icons / clusters | No — W6 |

Deck oracle (`VITE_WORLDMAP_ENABLED=1`): glyphs via `treeStore` + `IconLayer` at zoom ≥ 0.

## What you are building

1. Atlas GPU upload (`world-glyphs.webp` + JSON, 28 keys).
2. Icon instance pipeline ≤ 20 B + UV table.
3. Tree/veg/prop (+ badge) streams with Deck LOD/size parity.
4. Wire into `WgpuTacticalMap` after forest outline.
5. Exhaustive LOD scan + GPU-R tree probe.

## Do not

- Edit docs/registry/CLAUDE.
- Change forest mass / landcover export thresholds.
- Implement slot ring / clusters / editor pick.
- Break W2–W4.1 regressions.

## Execution order

1. Atlas + UV table + pipeline scaffold.
2. Size/LOD pure ports + exhaustive scan tests.
3. Stream hook (mirror `treeStore` policy).
4. Badges + prefs + draw order.
5. GPU-R + verify log; tag **T-151.5**.

## Preflight

```bash
cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
test "$(git rev-parse --show-toplevel)" = "$(pwd)"
git status --porcelain
git lfs pull && make map-assets-link
cd apps/website/frontend && npm ci && cd ../../..
make wasm
```

Confirm atlas files exist under `apps/website/frontend/public/map-assets/glyphs/atlas/`.

## Key files

| Concern | Path |
|---------|------|
| Atlas loader (oracle) | `layers/worldGlyphAtlas.ts` |
| Glyph builders | `worldmap/treePropLayer.ts` |
| Stream store | `worldmap/treeStore.ts` |
| LOD | `worldmap/lodGates.ts` |
| Deck assembly | `worldmap/useWorldMapLayers.ts` |
| Badges | `worldmap/buildingLayer.ts` |
| W4 mount pattern | `wgpu/useWgpuForestMass.ts`, `WgpuTacticalMap.tsx` |
| Engine | `crates/map-engine-render/src/engine.rs` |
| Atlas assets | `public/map-assets/glyphs/atlas/world-glyphs.{webp,json}` |

## Gotchas

- **IconLayer ≠ binary buffers** — Deck uses object arrays; wgpu can use packed instances (that’s the point).
- **Zoom &lt; 0:** no tree glyphs — forest mass only (LOD5).
- **Budget:** visible set capped (`INSTANCE_BUDGET`); do not upload all 501k trees every frame.
- **Yaw:** export clockwise → negate for screen CCW (`deckAngleForRotationDeg`).
- **Forest analysis:** leave mass alone; operator will compare glyphs vs green fill after ship.

## Return

- SHA + tag **T-151.5**
- `.ai/artifacts/t151_5_verify_log.md`
- **Ready for Cursor doc sync.**
