# T-090.1.2.8 — Claude Code handoff (unified satellite texture)

**Slice:** T-090.1.2.8 · **Executor:** claude-code  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_8_unified_satellite_texture.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_8_unified_satellite_texture.md)

---

## Context

**T-090.1.2.4 @ `0d6fe485` — P0 FAIL.** No engine ortho API. **SAP + `.2.2` bridge is the source.**

This slice fixes **Reforger zoom feel** (GPU mips, one fetch) — **not** the ~256 m grid at max MC zoom.

---

## Input

- `packages/map-assets/everon/staging/sap/everon-sap-ortho.png` (12800²)
- Or rebuild mips from pyramid via existing `build-tile-pyramid.sh` logic inverted into bundle

---

## Deliverables

1. `.ai/artifacts/t090_1_2_8_format_spike.json`
2. `scripts/map-assets/build-unified-satellite.mjs`
3. `scripts/map-assets/verify-unified-satellite.mjs`
4. Frontend: unified texture loader + `useTerrainBasemapLayer` branch
5. `manifest.tiles.satellite.delivery: "unified"`
6. `.ai/artifacts/t090_1_2_8_verify_log.md`

---

## Return

Tag **T-090.1.2.8** · **Ready for Cursor doc sync.**
