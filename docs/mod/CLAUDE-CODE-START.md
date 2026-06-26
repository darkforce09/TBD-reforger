# Claude Code — mod / Workbench entry

**Canonical context:** [`../../CLAUDE.md`](../../CLAUDE.md) at the monorepo root.

**Mod / Workbench queue:** [`../../docs/TICKET_MOD_QUEUE.md`](../../docs/TICKET_MOD_QUEUE.md)

**T-068 MCP slices:** T-068.1 (registry export), T-068.5 (mod equip), T-068.8 (compat export) — spec hub [`t068_virtual_arsenal_program.md`](../specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md).

**Workspace:** monorepo root (`TBD-Reforger/`). Mod scripts live under `scripts/mod/`; run from repo root:

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/deploy-staging.sh --dry-run
```

**Rules:** production mod = `apps/mod/tbd-framework/` only; **never guess Enfusion APIs or ResourceNames** — use **enfusion-mcp** first.

---

## Workbench MCP setup (Claude Code runs this)

`tbd-dev-bootstrap.sh` is the **single entrypoint** — Claude Code runs it at the start of every T-068.1 / .5 / .8 slice. It:

1. Builds MCP pak symlink farm (`setup-mcp-game-root.sh`)
2. Copies gitignored `EnfusionMCP/` handlers from the `enfusion-mcp` npm package
3. **Launches Workbench** if Net API port **5775** is closed (`steam -applaunch 1874910`, wait up to **180s**)
4. Runs `wb_connect` + `mod_validate`

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
```

**Human only if** bootstrap prints `ACTION REQUIRED` (exit 1): open `apps/mod/tbd-framework/addon.gproj` in Workbench, enable **Net API**, then Claude Code re-runs bootstrap.

Expect ~19 `.c` files under `EnfusionMCP/` after first run. Staging deploy excludes this tree.

| Method | When |
|--------|------|
| **`bash scripts/mod/mcp-call.sh <tool> '<json>'`** | Claude Code **terminal** (always works) |
| Copy [`apps/mod/.mcp.json`](../../apps/mod/.mcp.json) → project `.mcp.json` | Optional native MCP tools in IDE session |
| Copy → [`.cursor/mcp.json`](../../.cursor/mcp.json) | Cursor IDE Workbench chats only |

Verify machine paths in `ENFUSION_GAME_PATH`, `ENFUSION_WORKBENCH_PATH`, `ENFUSION_PROJECT_PATH`.

**Smoke** (after bootstrap exit 0):

```bash
bash scripts/mod/mcp-call.sh wb_connect '{}'
bash scripts/mod/mcp-call.sh mod_validate '{"modPath":"'"$PWD"'/apps/mod/tbd-framework"}'
```

If `wb_connect` fails: reload `tbd-framework` in Workbench Resource Browser and retry.

---

## T-068.1 typical MCP flow

```
tbd-dev-bootstrap.sh  (auto-launch Workbench if needed)
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

## T-068.5 verify flow (shipped @ `21ec91e`)

```
tbd-dev-bootstrap.sh
→ cp /tmp/loadout-export.json → $PROFILE/TBD_LoadoutTest.json   # T-068.4 canonical JSON
→ wb_reload → wb_play → mcp-wb-logs.sh | grep Loadout → wb_stop
→ sha256sum "$PROFILE/TBD_LoadoutTest.json"
```

**Component:** `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c` on `TBD_GameMode.et`.

**Winning equip API (verified):**
- clothing → `TryInsertItem(item, EStoragePurpose.PURPOSE_LOADOUT_PROXY)`
- primary → `EquipWeapon(item)`

**Base body:** `{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et` @ 6400/6400.

**New `.c` file:** Workbench **cold restart** required (not just `wb_reload`) before class registers.

Profile layout: [`scripts/mod/setup-server-profile.sh`](../../scripts/mod/setup-server-profile.sh).  
Workbench `$profile:` → Proton pfx `…/ArmaReforgerWorkbench/profile/` (paste exact path in verify).
