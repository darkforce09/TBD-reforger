# T-091.0 — DEM + tile export (MCP / Claude Code)

**Ticket:** T-091 · **Slice:** T-091.0  
**Status:** Spec ready — **next Claude Code slice after T-090.0 docs ship**  
**Executor:** **claude-code** (+ **enfusion-mcp** / Workbench — same pattern as T-068.1)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`CLAUDE-CODE-START.md`](../../mod/CLAUDE-CODE-START.md)

> **No manual-only gate.** Claude Code runs bootstrap, Workbench/MCP export, anchor probes, verify scripts, and commits assets to `main`. Human only if bootstrap exits 1 (Net API / addon load — same as T-068.1).

---

## In one sentence

Automate Everon 16-bit DEM PNG + aligned tile pyramid export via Workbench/MCP, probe ≥10 anchors with `GetSurfaceY`, commit under `packages/map-assets/everon/`, and pass `make verify-terrain-strict`.

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-090.0** | Manifest schema, verify scripts, DEV_RUNBOOK on disk |
| **Bootstrap** | `bash scripts/mod/tbd-dev-bootstrap.sh` exit 0 |
| **Git LFS** | `.gitattributes` in repo; `git lfs install` before PNG/WebP commit |

---

## Problem

No DEM or tiles in repo — editor Z is always 0; horizontal alignment unverified; T-090.1 and T-091.1 blocked.

---

## Goal

1. Export **16-bit heightmap PNG** (Base **and** Modified for A/B test).
2. Export **tile pyramid** `tiles/{z}/{x}/{y}.webp`.
3. Update `manifest.json` with **measured** `widthPx`, `heightPx`, height range, `exportedAt`, `workbenchVersion`, chosen `dem.source`.
4. Automate ≥10 `GetSurfaceY(x,z)` probes → `anchors/verification.json` (+ optional `demYM` pre-fill after decode script lands).
5. `make verify-terrain-strict` exit 0; commit on `main`.

---

## Out of scope

- Frontend DEM code (**T-091.1**)
- Z on place/move (**T-091.2**)
- Arland export (defer until Everon gate PASS)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Automation | **Claude Code + enfusion-mcp** — not a human-only slice |
| Bootstrap | Always start with `tbd-dev-bootstrap.sh` (auto-launch Workbench) |
| PNG encoding | uint16 linear: `elevM = min + (uint16/65535) * (max - min)` |
| Base vs Modified | Export **both**; pick lower max anchor error vs `GetSurfaceY` |
| Anchor threshold | Initial **±1.0 m** — tune in anchors `thresholdM` if needed |
| World path | MCP / Workbench must **record exact path** in ops log — never guess |
| Pixel dimensions | From **Info & Diags** via MCP or UI read — not assumed 6400² |
| Human fallback | Only when bootstrap `ACTION REQUIRED` (Net API, addon not loaded) |

---

## Claude Code MCP flow (primary path)

Same entrypoint as T-068.1 — see [`CLAUDE-CODE-START.md`](../../mod/CLAUDE-CODE-START.md).

```text
bash scripts/mod/tbd-dev-bootstrap.sh          # exit 0 required
→ wb_connect
→ mod_validate
→ [discover Everon world path via MCP — record in ops log]
→ [Terrain Creation Tool: Info & Diags → manifest fields]
→ [Export heightmap Base + Modified PNG to /tmp or repo staging]
→ [Export / generate tile pyramid → packages/map-assets/everon/tiles/]
→ wb_play @ anchor coordinates (≥10 points)
→ mcp-wb-logs.sh / game_read → extract GetSurfaceY → verification.json
→ [optional: run dem decode sample to fill demYM on anchors]
→ make verify-terrain && make verify-terrain-strict
→ cd packages/tbd-schema && npm run validate
→ git add packages/map-assets/ … && commit on main (tag T-091.0)
```

### MCP tools (use — do not guess APIs)

| Step | MCP / script |
|------|----------------|
| Connect | `mcp-call.sh wb_connect '{}'` |
| Validate mod | `mcp-call.sh mod_validate '{"modPath":"…/tbd-framework"}'` |
| Asset / world discovery | `asset_search`, `game_browse`, `game_read` |
| Playtest anchors | `wb_play` → grep logs for surface Y / spawn traces |
| Logs | `scripts/mod/mcp-wb-logs.sh` |
| Stop | `wb_stop` |

If World Editor heightmap export is not exposed via MCP today, Claude Code may:

1. Use **Workbench UI automation path** documented in ops log (export dialog settings recorded verbatim), **or**
2. Use [Enhanced Map Tool](https://github.com/Til-Weimann/tilw-terrain-tools) CLI from repo script (add `scripts/website/tile-pyramid.sh` when needed), **or**
3. Implement a **`TBD_TerrainExportPlugin.c`** in `tbd-framework` (same pattern as `TBD_RegistryItemsExportPlugin.c`) and invoke via MCP reload + export.

**Do not** hand-author heightmap pixels or anchor `surfaceYM` values.

---

## Anchor set (minimum ≥10)

| id | Source |
|----|--------|
| `coast-sw`, `valley-inland`, `hill-north`, `airfield` | MCP probe @ representative Everon coords |
| `bridgehead-1` … `bridgehead-3+` | x/z from [`bridgehead-at-levie.json`](../../../packages/tbd-schema/golden-missions/bridgehead-at-levie.json) slots |

Write [`anchors/verification.json`](../../../packages/map-assets/everon/anchors/verification.json) — schema [`terrain-anchors.schema.json`](../../../packages/tbd-schema/schema/terrain-anchors.schema.json).

---

## Workbench reference (when MCP needs UI)

Official path: [Terrain Creation Tool wiki](https://community.bohemia.net/wiki/Arma_Reforger:World_Editor:_Terrain_Creation_Tool)

1. Open Everon → Terrain Creation Tool → **Info & Diags** (record all values).
2. **Manage → Height Map → Rebuild Height Map**
3. **Export Height Map** → 16-bit PNG (Base + Modified)
4. Save winner → `packages/map-assets/everon/dem/everon-dem-16bit.png`
5. Tiles: satellite / 2D map export or Enhanced Map Tool → `tiles/{z}/{x}/{y}.webp`

---

## Verification gate (mandatory)

**Advance T-091.0 → T-090.1 / T-091.1 only when ALL PASS.**

### Automated (exit 0 — Claude Code runs these)

```bash
bash scripts/mod/tbd-dev-bootstrap.sh   # preflight
make verify-terrain
make verify-terrain-strict
make schema-validate
test -f packages/map-assets/everon/dem/everon-dem-16bit.png
test -d packages/map-assets/everon/tiles/0
jq -e '.dem.widthPx > 0 and .dem.heightPx > 0' packages/map-assets/everon/manifest.json
jq -e '.anchors | length >= 10' packages/map-assets/everon/anchors/verification.json
```

### Acceptance criteria (A1–A10)

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | DEM file | 16-bit PNG in repo; manifest dims **match** Info & Diags |
| A2 | Height range | Manifest min/max matches export (includes negatives) |
| A3 | Anchors | All entries ±1.0 m (`make verify-terrain-strict`) |
| A4 | Base vs Modified | Ops log documents both errors; winner in `dem.source` |
| A5 | Tiles | z0 directory exists; sample tile opens |
| A6 | H1 | Grid origin ↔ map corner documented in ops log |
| A7 | H2 | 3 zoom levels landmark check documented |
| A8 | LFS | PNG/WebP tracked; `git lfs pull` restores files |
| A9 | Schema | `make schema-validate` exit 0 |
| A10 | Git | Assets committed on `main` (no feature branch) |
| A11 | MCP evidence | Ops log includes bootstrap + wb_connect + probe log paths |

---

## Ops log template (Claude Code fills — commit under `.ai/artifacts/` or PR notes)

```text
Date:
Claude Code session / commit:
Workbench version:
Everon world path (exact — from MCP):
Info & Diags — planar resolution (m):
Info & Diags — height min/max (m):
Info & Diags — heightmap widthPx × heightPx:
Export method (MCP plugin / WE UI / EMT):
Export — Base PNG max anchor error (m):
Export — Modified PNG max anchor error (m):
Chosen dem.source:
Tile export tool:
MCP probe log path:
make verify-terrain-strict: pass/fail
Git LFS push: yes/no
```

---

## Claude Code prompt stub

```text
Read t091_0_dem_tile_export.md and CLAUDE-CODE-START.md.
DO NOT edit documentation.
Run tbd-dev-bootstrap.sh; export Everon DEM + tiles; probe ≥10 anchors via wb_play;
write verification.json; make verify-terrain-strict; commit assets on main tag T-091.0.
Return: verify output, ops log, sha256 of DEM PNG, anchor count.
```

---

## Related

- [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md)
- [`t091_1_dem_loader.md`](t091_1_dem_loader.md)
