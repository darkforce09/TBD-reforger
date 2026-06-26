# T-068.1 — Workbench flat ResourceName export (MCP-driven)

**Ticket:** T-068 · **Slice:** T-068.1  
**Status:** Spec ready — code pending  
**Executor:** claude-code (**enfusion-mcp required**)  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

**Agent roles (locked):** **Claude Code** implements export + commits JSON using **enfusion-mcp** for real prefab data. **Human** only runs Workbench preflight (launch Tools, Net API, bootstrap) — not hand-authoring 20+ ResourceNames.

---

## In one sentence

Use **enfusion-mcp** + mod export tooling to emit validated `registry-items` JSON (characters + Phase 1 gear) with real Enfusion ResourceNames.

---

## Why MCP (not optional)

Mock catalog / guessed GUIDs are **invalid** for Phase 1. The web palette and loadout dropdowns need **`resource_name` strings that exist in vanilla Reforger**. Claude Code must:

1. **Discover** prefab paths via MCP (`asset_search`, `game_read`, `game_browse`) against the pak symlink farm ([`scripts/mod/setup-mcp-game-root.sh`](../../../scripts/mod/setup-mcp-game-root.sh)), **or**
2. **Implement + run** a Workbench export script validated with `wb_reload` + `mod_validate`, then read the emitted file.

Manual “Copy Resource Name” for every row is a **spot-check only** (acceptance A7), not the primary data path.

---

## Human preflight (before Claude Code)

```bash
# 1. Launch Arma Reforger Tools from Steam; open tbd-framework addon.gproj
# 2. File > Options > General — enable Net API (default port 5775)
bash scripts/mod/tbd-dev-bootstrap.sh   # MCP root + EnfusionMCP handlers + wb_connect smoke
```

Copy [`apps/mod/.cursor/mcp.json`](../../../apps/mod/.cursor/mcp.json) → repo-root `.cursor/mcp.json` **or** Claude Code `.mcp.json` if the session supports MCP tools. Paths in that file must match this machine (`ENFUSION_*`).

If Workbench is not running, Claude Code **stops** and reports “preflight FAIL” — do not fabricate export JSON.

---

## Problem

API and palette need real Enfusion ResourceNames; mock catalog uses fake ids.

---

## Goal

1. Extend mod export (Workbench plugin or game script) to emit `registry-items` envelope per T-068.0.1 schema.
2. Populate rows using **MCP-discovered** or **Workbench-exported** ResourceNames — categories `NATO/Men/...`, `NATO/Gear/Primary/...`, etc.
3. Include ≥20 rows: US rifleman/SL/medic + sample primary/uniform/vest/helmet ResourceNames.
4. Commit export to e.g. `packages/tbd-schema/registry/registry-items.workbench.json` (path documented in verify paste).
5. `npm run validate` includes the workbench export file.

---

## MCP workflow (Claude Code)

| Step | Tool / script | Purpose |
|------|---------------|---------|
| Connect | `bash scripts/mod/mcp-call.sh wb_connect '{}'` | Net API session |
| Discover | MCP `asset_search` / `game_read` / `game_browse` (or `mcp-call.sh` equivalent) | Find character + gear prefab ResourceNames |
| Implement | Edit `apps/mod/tbd-framework/` export script | Emit `registry-items` JSON |
| Validate mod | `mcp-call.sh mod_validate '{"modPath":"…/tbd-framework"}'` | Scripts compile in Workbench context |
| Reload | `mcp-call.sh wb_reload '{"scope":"scripts"}'` | Pick up new `.c` files |
| Export | Run export (MCP tool or Workbench action documented in paste) | Produce committed JSON |
| Schema | `cd packages/tbd-schema && npm run validate` | Gate A2 |

Never guess Enfusion APIs or GUIDs — [`docs/mod/CLAUDE-CODE-START.md`](../../mod/CLAUDE-CODE-START.md).

---

## Out of scope

- Postgres ingest (T-068.2 `import-registry-items`)
- Compat edges (T-068.8)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Format | `registry-items.schema.json` envelope |
| Data source | **MCP + Workbench export** — not hand-typed JSON |
| Vanilla modset | Arma Reforger base + TBD framework |
| Fallback for T-068.2 smoke | `registry_dev.sql` can ship before this slice; **T-068.6 E2E prefers this export** |

---

## Tasks

1. MCP discovery pass — document search queries + hit count in verify paste
2. Export script/component in `apps/mod/tbd-framework/`
3. Committed `registry-items.workbench.json` (≥20 rows, all kinds)
4. `validate.mjs` includes workbench export path

---

## Verify

```bash
# Preflight (must PASS before export work)
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/mcp-call.sh wb_connect '{}'

cd packages/tbd-schema && npm run validate
EXPORT=registry/registry-items.workbench.json
test -f "$EXPORT"
jq -e '.items | length >= 20' "$EXPORT"
jq -e '[.items[].kind] | unique | inside(["character","gear_primary","gear_uniform","gear_vest","gear_helmet"])' "$EXPORT"
jq -e '[.items[] | select(.kind=="character")] | length >= 5' "$EXPORT"
jq -e '[.items[] | select(.kind=="gear_primary")] | length >= 1' "$EXPORT"
jq -e '[.items[] | select(.kind=="gear_uniform")] | length >= 1' "$EXPORT"
jq -e '[.items[] | select(.kind=="gear_vest")] | length >= 1' "$EXPORT"
jq -e '[.items[] | select(.kind=="gear_helmet")] | length >= 1' "$EXPORT"
jq -e '[.items[].resource_name | test("^\\{[0-9A-F]{16}\\}")] | all' "$EXPORT"
```

---

## Verification gate (mandatory)

**Advance when ALL PASS** (may run parallel to T-068.2 after T-068.0.1).

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A0 | MCP preflight | `tbd-dev-bootstrap.sh` + `wb_connect` exit 0 (paste output) |
| A1 | Export file exists | Committed path documented in paste |
| A2 | Schema valid | `npm run validate` exit 0 including workbench export |
| A3 | Row count | `items.length >= 20` |
| A4 | Kinds coverage | ≥5 `character`, ≥1 each gear kind |
| A5 | ResourceName | 100% items match GUID ResourceName regex |
| A6 | Categories | ≥3 distinct `category` paths with `/` segments |
| A7 | Workbench spot-check | Paste 3 `resource_name` values — MCP/search source **or** Workbench “Copy Resource Name” matches export rows |
| A8 | MCP evidence | Paste ≥1 `asset_search` / `game_read` query + truncated result showing a row’s `resource_name` |

### Verify paste (required)

Program hub template + **A0**, **A7**, **A8**.

---

## Depends on / Unblocks

- **Depends on:** T-068.0.1
- **Unblocks:** T-068.6 (real data); optional `import-registry-items` before E2E

---

## Documentation sync (Cursor)

After verify paste: note export path in T-068.6 checklist if changed.

---

## Claude Code prompt — T-068.1

```
Read CLAUDE.md §Status. Active slice: T-068.1.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_1_workbench_flat_export.md

PREFLIGHT (human must have Workbench running — you verify):
  bash scripts/mod/tbd-dev-bootstrap.sh
  bash scripts/mod/mcp-call.sh wb_connect '{}'
If preflight FAILs, stop and report — do NOT invent ResourceNames.

Use enfusion-mcp (mcp-call.sh or session MCP) to discover vanilla character + gear prefabs.
Implement export in apps/mod/tbd-framework/; commit packages/tbd-schema/registry/registry-items.workbench.json.
Wire validate.mjs. Do not edit documentation. Branch: ticket/T-068

Verify: all §Verification gate commands; acceptance A0–A8.
Return: Verify paste + MCP discovery log snippet + export path.
```
