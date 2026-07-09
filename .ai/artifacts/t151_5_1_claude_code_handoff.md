# T-151.5.1 — Claude Code handoff (forest mass / landcover fidelity)

**Spec (wins on conflict):**
[`t151_5_1_forest_fidelity.md`](../../docs/specs/Mission_Creator_Architecture/t151_5_1_forest_fidelity.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `0b7621ed` (tag **T-151.5**) — **never `main`**.

## Operator report

T-151.5 glyphs prove the green layers are wrong: overshoot past trees, gap bridging, huge
envelopes on thin stands, and a 32 m “grid” inside the fill when zoomed in. Fix **runtime + LOD**
now; export rebuild / T-149 smoothing later.

## CURRENT STATE

| Layer @ zoom −2 | Layer @ zoom ≥ 0 |
|-----------------|------------------|
| Forest fill + outline + landcover ON (iso=1, bloated) | Same green + **tree glyphs** on top |
| Tree glyphs OFF | Glyphs ON — green still bloated underneath |

## What you are building

1. `DENSITY_ISO = 2` (TS + Rust).
2. Hide fill + outline + landcover when tree glyph band active (`zoom ≥ 0`).
3. wgpu landcover visibility refresh on zoom.
4. Deck parity for the same gates.
5. Verify log; tag **T-151.5.1**.

## Do not

- Rebuild Path B / TBDD assets.
- Edit docs/registry.
- Touch T-151.6 slots.
- Leave iso=1 “for parity with old goldens” — update tests instead.

## Key files

| Concern | Path |
|---------|------|
| Iso + march | `worldmap/forestMass.ts`, `geometry/forest_mass.rs` |
| LOD | `worldmap/lodGates.ts`, `world/lod_gates.rs` |
| Deck layers | `forestMassLayer.ts`, `landCoverRegions.ts`, `useWorldMapLayers.ts` |
| wgpu | `useWgpuForestMass.ts`, `wgpuWorldLoader.ts` (`landcoverPushed`) |
| Parity | `features/_wasm/forest.parity.test.ts` |

## Gotchas

- Comment in `forestMass.ts` already says region export used threshold **2** — iso=1 was the bug.
- “Grid” = per-cell marching squares, not `LaneRole::Grid`.
- Coarse zoom must still show *some* forest context without glyphs.
- Mega-region `forest-everon-001` may still look large at −2 after iso=2 — document, don’t rebuild.

## Return

- SHA + tag **T-151.5.1**
- `.ai/artifacts/t151_5_1_verify_log.md`
- **Ready for Cursor doc sync.**
