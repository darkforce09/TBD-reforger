# T-121 — Terrain export follow-ons (tiles, Arland, tooling)

**Ticket:** T-121 · **Status:** **deferred**  
**Executor:** **claude-code** (+ MCP where needed)  
**Depends on:** **T-091.0** shipped @ `6d96339`  
**Authority:** [`.ai/artifacts/t091_0_ops_log.txt`](../../../.ai/artifacts/t091_0_ops_log.txt)

---

## In one sentence

Follow-on automation after T-091.0: tile pyramid (EMT), Arland DEM re-export, and Workbench/MCP tooling polish — **not** the initial Everon DEM gate (that shipped via `TBD_TerrainExportPlugin.c`).

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

1. **Tile pyramid** — Enhanced Map Tool / WE Export Map data → `tiles/{z}/{x}/{y}.webp` + V5 ops log (**T-090.1** may own this instead).
2. **Arland DEM** — repeat plugin export + manifest + anchors.
3. **MCP helper hardening** — `mcp-call.sh` round-trip reliability (session drops noted @ T-091.0).
4. **Optional:** game-mode export fallback (`TBD_HeightmapExportComponent`) if Workbench plugin context regresses on future engine versions.

---

## Out of scope

- Replacing committed Everon DEM without re-running `make verify-terrain-strict`
- Re-opening manual Eden Terrain Tool export (dead path)

---

## Related

- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — shipped Everon gate
- [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)
