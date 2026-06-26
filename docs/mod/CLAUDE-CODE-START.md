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

## Workbench MCP setup (every clone)

1. **`EnfusionMCP/` handlers** — gitignored; copied by bootstrap:

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
```

Expect ~19 `.c` files under `apps/mod/tbd-framework/Scripts/WorkbenchGame/EnfusionMCP/`. Staging deploy excludes this tree.

2. **Launch Workbench** — Arma Reforger Tools from Steam; open `apps/mod/tbd-framework/addon.gproj`; enable **Net API** (File → Options → General, port **5775** default).

3. **MCP invocation** (pick one):

| Method | When |
|--------|------|
| **`bash scripts/mod/mcp-call.sh <tool> '<json>'`** | Claude Code **terminal** (always works) |
| Copy [`apps/mod/.mcp.json`](../../apps/mod/.mcp.json) → project `.mcp.json` | Claude Code / Cursor session with native MCP tools |
| Copy → [`.cursor/mcp.json`](../../.cursor/mcp.json) | Cursor IDE Workbench chats |

Verify machine paths in `ENFUSION_GAME_PATH`, `ENFUSION_WORKBENCH_PATH`, `ENFUSION_PROJECT_PATH`. Pak symlink farm: [`scripts/mod/setup-mcp-game-root.sh`](../../scripts/mod/setup-mcp-game-root.sh).

4. **Smoke connect:**

```bash
bash scripts/mod/mcp-call.sh wb_connect '{}'
bash scripts/mod/mcp-call.sh mod_validate '{"modPath":"'"$PWD"'/apps/mod/tbd-framework"}'
```

If `wb_connect` fails: reload `tbd-framework` in Workbench Resource Browser and retry.

---

## T-068.1 typical MCP flow

```
wb_connect → asset_search / game_read / game_browse  (discover prefabs)
→ implement export script in tbd-framework
→ wb_reload → mod_validate → run export
→ commit packages/tbd-schema/registry/registry-items.workbench.json
→ npm run validate in packages/tbd-schema
```

Human runs Workbench + bootstrap; **Claude Code drives MCP** — do not hand-author 20+ GUIDs.

After export: **`go run ./cmd/import-registry-items`** (T-068.2) loads rows into Postgres before Phase 1 E2E.

---

## T-068.5 verify flow

```
Copy web loadout-export.json → $profile:TBD_LoadoutTest.json
→ wb_reload → wb_play → mcp-wb-logs.sh (equip lines) → wb_stop
```

Profile layout: [`scripts/mod/setup-server-profile.sh`](../../scripts/mod/setup-server-profile.sh).
