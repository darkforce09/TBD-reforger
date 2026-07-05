# Claude Code — mod / Workbench entry

**Canonical context:** [`../../CLAUDE.md`](../../CLAUDE.md) at the monorepo root.

**Mod / Workbench queue:** [`../../docs/TICKET_MOD_QUEUE.md`](../../docs/TICKET_MOD_QUEUE.md)

**T-068 MCP slices:** T-068.1 (registry export), T-068.5 (mod equip), T-068.8 (compat export) — spec hub [`t068_virtual_arsenal_program.md`](../specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md).

**T-091.0 (shipped @ `6d96339`):** Everon 6400² DEM via `TBD_TerrainExportPlugin.c` + strict verify — spec [`t091_0_dem_tile_export.md`](../specs/Mission_Creator_Architecture/t091_0_dem_tile_export.md).

**T-121 (deferred):** Arland re-export + optional game-mode fallback — MCP hardening **shipped** @ `e7e7232` — spec [`t121_terrain_dem_export_automation.md`](../specs/Mission_Creator_Architecture/t121_terrain_dem_export_automation.md).

**T-090.3.0 (shipped @ `b342c35`):** Workbench spike — enumeration + OBB + forest/handedness findings. Ops log [`.ai/artifacts/map_export_everon.json`](../../.ai/artifacts/map_export_everon.json). Harness: `scripts/map-assets/verify-spike-*.mjs`.

**Next Claude Code:** **T-090.5.3** only — worker chunk streaming (`worldObjects.worker.ts` full W1–W5), `chunkStore` LRU/budget, `visibleInstances`. Single lane. Plan §7 row T-090.5.3 · spec [`t090_5_map_object_render_layer.md`](../specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md).

**Workspace:** monorepo root (`TBD-Reforger/`). Mod scripts live under `scripts/mod/`; run from repo root:

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/deploy-staging.sh --dry-run
```

**Rules:** production mod = `apps/mod/tbd-framework/` only; **never guess Enfusion APIs or ResourceNames** — use **enfusion-mcp** first.

---

## Workbench MCP setup (Claude Code runs this)

**Full reference:** [`MCP_TOOLING.md`](MCP_TOOLING.md)

`tbd-dev-bootstrap.sh` is the **single entrypoint** — Claude Code runs it at the start of every mod slice. It:

1. Builds MCP pak symlink farm (`setup-mcp-game-root.sh`)
2. Runs `npm ci` in `scripts/mod/` when `enfusion-mcp` is not installed (pinned @ 0.6.1)
3. Copies gitignored `EnfusionMCP/` handlers from the npm package
4. **Launches Workbench** if Net API port **5775** is closed (`steam -applaunch 1874910`, wait up to **180s**)
5. **Pre-warms the MCP daemon** (one-time ~35 s index load)
6. Runs `wb_connect` + `mod_validate`

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
```

**Human only if** bootstrap prints `ACTION REQUIRED` (exit 1): open `apps/mod/tbd-framework/addon.gproj` in Workbench, enable **Net API**, then Claude Code re-runs bootstrap.

Expect ~19 `.c` files under `EnfusionMCP/` after first run. Staging deploy excludes this tree.

| Method | When |
|--------|------|
| **`bash scripts/mod/mcp-call.sh <tool> '<json>'`** | Claude Code **terminal** (daemon-first; warm ~0.3 s) |
| Copy [`apps/mod/.mcp.json`](../../apps/mod/.mcp.json) → project `.mcp.json` | Optional native MCP tools in IDE session |
| Copy → `.cursor/mcp.json` (gitignored — local only) | Cursor IDE Workbench chats only |

Verify machine paths in `ENFUSION_GAME_PATH`, `ENFUSION_WORKBENCH_PATH`, `ENFUSION_PROJECT_PATH`.

**Smoke** (after bootstrap exit 0):

```bash
bash scripts/mod/mcp-smoke.sh
bash scripts/mod/mcp-call.sh mod_validate '{"modPath":"'"$PWD"'/apps/mod/tbd-framework"}'
```

**Offline self-test** (no Workbench):

```bash
bash scripts/mod/mcp-call-selftest.sh   # 19/19 gates
```

**Clean slate** if processes leak or load spikes:

```bash
bash scripts/mod/mcp-daemon.sh stop-all
```

If `wb_connect` fails: reload `tbd-framework` in Workbench Resource Browser and retry.

---

## T-068.1 typical MCP flow

```
tbd-dev-bootstrap.sh  (auto-launch Workbench + daemon pre-warm)
→ wb_connect → asset_search / game_read / game_browse
→ implement export script in tbd-framework
→ wb_reload → mod_validate → run export
→ commit packages/tbd-schema/registry/registry-items.workbench.json
→ npm run validate in packages/tbd-schema
```

Do not hand-author 20+ GUIDs. After export, upsert into Postgres from **`apps/website`** module root:

```bash
cd apps/website
export PATH="$HOME/.local/go/bin:$PATH"
go run ./cmd/import-registry-items \
  --file ../../packages/tbd-schema/registry/registry-items.workbench.json
```

(`make seed` applies `registry_dev.sql` for local API smoke without Workbench.) See [`DEV_RUNBOOK.md`](../website/DEV_RUNBOOK.md) §Registry catalog.

---

## T-068.5 / T-068.5.1 verify flow (Phase 1 shipped)

```
tbd-dev-bootstrap.sh
→ cp /tmp/loadout-export.json → $PROFILE/TBD_LoadoutTest.json   # T-068.4 download
→ wb_reload → wb_play → mcp-wb-logs.sh | grep Loadout → wb_stop
→ sha256sum "$PROFILE/TBD_LoadoutTest.json"
→ screenshot: test NPC @ spawn shows kit (not human player)
```

**Component:** `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c` on `TBD_GameMode.et`.

**Phase 1 subject:** **Non-player test NPC** @ game-mode coords (~6400). **Not** `DeployPlayer` / human player.

**Winning equip API (@ T-068.5.1 `b233b11`):**
- clothing → `EquipCloth(item)` + worn-verify via `GetClothFromArea(LoadoutJacketArea|LoadoutVestArea|LoadoutHeadCoverArea)`
- primary → `EquipWeapon(item)` + verify via `GetCurrentWeapon()`

**Scaffold (superseded @ T-068.5.1):** `TryInsertItem(PURPOSE_LOADOUT_PROXY)` logged OK but left NPC naked.

**Base body:** `{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et` @ 6400/6400.

**Player loadout from editor:** **T-068.11** — not Phase 1.

---

## T-090.1.2 — active (SAP supertexture satellite detail)

**Spec:** [`t090_1_2_sap_supertexture_satellite.md`](../specs/Mission_Creator_Architecture/t090_1_2_sap_supertexture_satellite.md)  
**Handoff:** [`.ai/artifacts/t090_1_2_claude_code_handoff.md`](../../.ai/artifacts/t090_1_2_claude_code_handoff.md)

**T-090.1 shipped** @ `564419e` — basemap + alignment + LOD proven. This slice replaces tile pixels with decoded `Eden_*_supertexture.edds` stitch.

## T-090.1 — shipped (interim satellite basemap) @ `564419e`

**Spec:** [`t090_1_aligned_basemap.md`](../specs/Mission_Creator_Architecture/t090_1_aligned_basemap.md) · verify: [`.ai/artifacts/t090_1_verify_log.md`](../../.ai/artifacts/t090_1_verify_log.md)

---

## T-091.2 — shipped (Z-axis editor UX) @ `dde589e`

**Spec:** [`t091_2_z_axis_editor.md`](../specs/Mission_Creator_Architecture/t091_2_z_axis_editor.md) · handoff (historical): [`.ai/artifacts/t091_2_claude_code_handoff.md`](../../.ai/artifacts/t091_2_claude_code_handoff.md)

## T-091.1 — shipped (DEM loader) @ `2c56c2e`

**Spec:** [`t091_1_dem_loader.md`](../specs/Mission_Creator_Architecture/t091_1_dem_loader.md) · handoff (historical): [`.ai/artifacts/t091_1_claude_code_handoff.md`](../../.ai/artifacts/t091_1_claude_code_handoff.md)

| Module | Path |
|--------|------|
| DEM modules | `src/features/tactical-map/dem/*` |
| Tests | `dem/sampleElevation.test.ts` (15 tests; 11 anchors ±0.01 m) |
| Wire | `TacticalMap.tsx` → `loadDemForTerrain` |
| Browser | `vite.config.ts` `pngjs→browser`; `buffer` in `DemTexture.ts` |

Verify: `npm test` + `make verify-terrain-strict`
