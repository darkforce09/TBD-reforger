# T-090.1.2 — Claude Code handoff (SAP supertexture satellite)

**Generated:** 2026-06-30 · **Executor:** claude-code · **Commit on:** `main` (`T-090.1.2:` prefix)  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_sap_supertexture_satellite.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_sap_supertexture_satellite.md)  
**Program hub:** [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)

---

## Context (read this first)

**T-090.1 is shipped** @ `564419e` — basemap works, aligned, pyramid LOD in Mission Creator. Source is interim **`MapDataExporter.ExportRasterization`** (stylized 4096² shaded map). Operator confirmed alignment OK; zoom detail insufficient.

**T-090.1.2** replaces tile **pixels only** with **SAP supertexture stitch** — decode `worlds/Eden/Eden/.Data/Eden_*_supertexture.edds`, composite world ortho, rebuild pyramid. Frontend already correct unless manifest source field changes.

**Not this slice:** roads/labels = **T-090.1.1** · map object glyphs = **T-090.5**.

---

## Preflight

```bash
bash scripts/mod/mcp-call-selftest.sh
bash scripts/mod/mcp-call.sh wb_connect '{}'   # if Workbench up
make map-assets-link
```

Pak farm: `~/.cache/enfusion-mcp-root` symlinks · Eden data under Steam addons `.pak`.

---

## Execution order (110% — no skip)

1. **P0 decode spike** — one `.edds` → PNG; write `.ai/artifacts/t090_1_2_decode_spike.json`. **STOP if fail.**
2. **P1 catalog** — all Eden supertexture cells → `cell-catalog.json`
3. **P2 stitch** — world ortho ≥8192² effective, north-up, `[0,0,12800,12800]`
4. **P3 pyramid** — `build-tile-pyramid.sh` (no flip unless proven needed) · LFS commit tiles
5. **P4 verify log** — D1–D4 + automated gates · tag `T-090.1.2`

---

## Do not redo

- `useTerrainBasemapLayer` / `tileUrl` / TacticalMap wiring (unless bug found)
- New Enfusion plugin unless decode via Workbench export is easier (prefer CLI decode)
- Docs/registry (Cursor sync after ship)

---

## Return to operator

1. Commit hash + `T-090.1.2` tag  
2. P0 spike JSON + sample decoded PNG path  
3. Ortho dimensions + total tile MB  
4. Copy-paste all automated verify output  
5. End: **"Ready for Cursor doc sync."**
