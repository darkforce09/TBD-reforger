# T-121 — Terrain export follow-ons (tiles, Arland, tooling)

**Ticket:** T-121 · **Status:** **deferred**  
**Executor:** **claude-code** (+ MCP where needed)  
**Depends on:** **T-091.0** shipped @ `6d96339`  
**Authority:** [`.ai/artifacts/t091_0_ops_log.txt`](../../../.ai/artifacts/t091_0_ops_log.txt)

---

## In one sentence

Follow-on automation after T-091.0: **DEM refresh**, Arland export, and Workbench/MCP tooling polish — **not** the initial Everon DEM gate (that shipped via `TBD_TerrainExportPlugin.c`). **Tile pyramid + world object export → T-090.3** (absorbed from T-121).

---

## Already shipped in T-091.0

| Item | Location |
|------|----------|
| Everon 6400² DEM PNG | `packages/map-assets/everon/dem/everon-dem-16bit.png` |
| GetSurfaceY resample plugin | `TBD_TerrainExportPlugin.c` |
| ASCII → PNG converter | `raw-u16-to-dem-png.mjs` |
| Schema enum | `dem.source`: `mod-getsurfacey-resample` |
| Strict verify pipeline | `dem-sample.mjs` + `verify-terrain-alignment.mjs` |

---

## T-121 scope (when picked up)

1. ~~**Tile pyramid**~~ → **T-090.3** [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md) (T-090.1 renders; T-090.3 exports).
2. **Arland DEM** — repeat plugin export + manifest + anchors.
3. ~~**MCP helper hardening**~~ — **shipped** @ `e7e7232`: warm daemon + hardened one-shot, pinned `enfusion-mcp@0.6.1`, offline self-test + live smoke. See [`docs/mod/MCP_TOOLING.md`](../../mod/MCP_TOOLING.md).
4. **Optional:** game-mode export fallback (`TBD_HeightmapExportComponent`) if Workbench plugin context regresses on future engine versions.

---

## Out of scope

- Replacing committed Everon DEM without re-running `make verify-terrain-strict`
- Re-opening manual Eden Terrain Tool export (dead path)

---

## Related

- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — shipped Everon gate
- [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)
