# T-150 — Universal Enfusion registry + compat export (mod-agnostic)

**Status:** **shipped** @ `9107bf4e` (tag **T-150**; schema commit `e358777a`) · **Program:**
Eden / Virtual Arsenal data lane · **Executor:** claude-code · **Verify:**
[`.ai/artifacts/t150_verify_log.md`](../../../.ai/artifacts/t150_verify_log.md) · **Next:**
**T-068.9** ingest + worker.

## In one sentence

Replace the curated 21-row Workbench export with a **mod-agnostic scanner** that, for
**whatever addons are loaded in Workbench**, emits (1) a full **items** catalog and (2) a
**compat edge** graph (weapon↔mag↔ammo, optic↔weapon, vehicle turret↔ammo, …) — no
hardcoded prefab lists, scales to any modset size.

## Problem

T-068.1 ships a **hand-curated** `BuildCuratedRows()` list (~21 vanilla characters/gear). That
cannot grow with Workshop mods, cannot express vehicle turret ammo wells, and cannot answer
“does this BTR take LAV ammo?”. Phase 2 arsenal and vehicle placement need **engine-derived**
census + compatibility for **every loaded addon**, not a TBD-maintained allowlist.

## Goal

1. **Schemas (Claude owns files under `packages/tbd-schema/`):**
   - Expand `registry-items.schema.json` `kind` enum (see Locked decisions).
   - Add `registry-compat.schema.json` (nodes = `resource_name`, typed edges).
   - Golden / sample fixtures that validate; bump `registryItemsVersion` / add
     `registryCompatVersion`.
2. **Workbench plugin (replace/extend T-068.1):**
   - Discover prefabs via **ResourceManager / addon enumeration** (or equivalent MCP-proven
     Enfusion API) across **all loaded projects/addons**.
   - Classify into kinds with **rules** (path + component type), not GUID tables.
   - Emit items JSON + compat JSON to `$profile:` (documented paths).
   - Skip static world junk (structures, fences, rocks) via denylist prefixes / missing
     inventory-or-weapon components — configurable constants, not per-mod lists.
3. **Compat edges (same export pass):** derive from engine data only, e.g.:
   - `mag_in_weapon`, `ammo_in_mag`, `optic_on_weapon`, `attachment_on_weapon`
   - `mag_in_vehicle_weapon` / `ammo_in_vehicle_weapon` (turret / mounted weapon wells)
   - `gear_in_slot` where discoverable (vest/uniform slot types) — if API missing, document
     explicit deferral in verify log (**no silent skip**).
4. **Commit** sample export from a vanilla (+ TBD framework) Workbench session under
   `packages/tbd-schema/registry/` (items + compat). Counts recorded in verify log.
5. **Validate:** `cd packages/tbd-schema && npm run validate`.
6. Tag **T-150**. Cursor doc-sync after return (hub / T-068.8 pointer).

## Out of scope

- Postgres ingest / `import-registry-items` expansion (**T-068.9**).
- Asset Browser wiring / map place vehicles (**T-146**, **T-070**).
- Smart Forge UI / `canEquip` worker (**T-068.10** / worker half of **T-068.9**).
- Markers (**T-069**), ORBAT Manager (**T-071**).
- Hand-authored “this gun takes these mags” tables.
- Perfect coverage of every Enfusion edge type on day one — ship the **architecture** +
  proven edge families above; name any missing family in the verify log.

## Locked decisions

| ID | Choice |
|----|--------|
| **Scale** | Export = **function of loaded addons**. Adding a Workshop mod to the Workbench project and re-running the plugin **must** include that mod’s matching prefabs/edges with **zero** code edits to curated lists. |
| **No curated GUID lists** | Delete / retire `BuildCuratedRows()` allowlists. Classification = path prefixes + component introspection. |
| **Identity** | Full Enfusion `resource_name` (`{GUID}Prefabs/.../File.et`) everywhere. |
| **Dual artifact** | One plugin run → **items** envelope + **compat** envelope (two files or one zip — document). |
| **modpackId** | Still UUID string in envelope; also record `addons[]` metadata (addon name / GUID / version if available) so ingest knows the scan set. |
| **Items `kind` enum (v2)** | At minimum: `character`, `gear_primary`, `gear_handgun`, `gear_launcher`, `gear_uniform`, `gear_vest`, `gear_helmet`, `gear_backpack`, `magazine`, `ammo`, `optic`, `attachment`, `vehicle`, `vehicle_weapon`, `crate`, `other` (escape hatch — count must be reported; prefer classifying). Extend schema enum accordingly; keep Phase 1 kinds valid. |
| **Compat edge types (v1)** | `mag_in_weapon`, `ammo_in_mag`, `optic_on_weapon`, `attachment_on_weapon`, `mag_in_vehicle_weapon`, `ammo_in_vehicle_weapon`. Optional if cheap: `character_default_loadout` (character → gear). |
| **Skip static** | Denylist path fragments e.g. `/Structures/`, `/Props/`, `/Rocks/`, `/Trees/`, `/Debris/` unless they also expose inventory/weapon components that force inclusion. |
| **API discovery** | **Never guess** Enfusion APIs — `api_search` / MCP / existing plugins first. If an edge family cannot be read from the engine, **stop that family**, log OPEN in verify, do not invent edges. |
| **T-068.8** | This slice **implements** the Workbench compat export T-068.8 described; after ship, Cursor marks T-068.8 shipped/superseded-by-T-150. |
| **Docs/registry** | Claude does **not** edit registry/CLAUDE/hub (verify log OK). |

## Tasks

1. MCP preflight: `bash scripts/mod/tbd-dev-bootstrap.sh` + `wb_connect` + `mod_validate`.
2. Spike: find ResourceManager / prefab enumeration + weapon/magazine/vehicle component APIs
   (`api_search`). Record findings in verify log.
3. Schema: expand `registry-items` kinds; add `registry-compat.schema.json` + goldens; wire
   `npm run validate`.
4. Implement scanner plugin (new class OK; retire curated list in old plugin or replace menu
   action). Menu: e.g. `Plugins,Export TBD Registry (Universal)`.
5. Run export against loaded vanilla (+ tbd-framework); copy artifacts into
   `packages/tbd-schema/registry/`; commit.
6. Prove scale claim: document “add addon X → re-run → counts increase” procedure in verify
   log (even if only vanilla this pass — architecture must not require code change).
7. `.ai/artifacts/t150_verify_log.md` + tag **T-150**.

## Verify

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/mcp-call.sh wb_connect '{}'
bash scripts/mod/mcp-call.sh mod_validate "{\"modPath\":\"$PWD/apps/mod/tbd-framework\"}"
# After export + copy into packages/tbd-schema/registry/:
cd packages/tbd-schema && npm run validate
# Counts (example — record real numbers):
node -e "const i=require('./registry/registry-items.workbench.json'); const c=require('./registry/registry-compat.workbench.json'); console.log('items',i.items.length); console.log('edges',(c.edges||c.compat||[]).length)"
```

Workbench: plugin completes without curated-list code paths; sample edges include at least one
`mag_in_weapon` and one vehicle-related edge **if** a vehicle with a weapon well exists in the
loaded set (else document OPEN).

## Acceptance

- [ ] No hardcoded prefab GUID/path allowlist drives the export.
- [ ] Items schema accepts new kinds; validate green.
- [ ] Compat schema exists; validate green; edges are engine-derived.
- [ ] Committed sample export under `packages/tbd-schema/registry/`.
- [ ] Verify log states how a new Workshop mod is included (load in WB → re-run plugin).
- [ ] Tag **T-150**.

## Claude Code prompt — T-150 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root (NOT a spike worktree).

Implement **T-150** — Universal Enfusion registry + compat export (mod-agnostic).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  git pull && git lfs pull
  bash scripts/mod/tbd-dev-bootstrap.sh
  bash scripts/mod/mcp-call.sh wb_connect '{}'
  bash scripts/mod/mcp-call.sh mod_validate "{\"modPath\":\"$PWD/apps/mod/tbd-framework\"}"

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t150_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t150_universal_registry_export.md
  3. docs/specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md
  4. docs/mod/CLAUDE-CODE-START.md
  5. apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c  (retire curated list)
  6. packages/tbd-schema/schema/registry-items.schema.json
  7. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  T-068.1 export is a hardcoded ~21-row allowlist. We need a scanner that works for every
  loaded mod (infinite modsets) and emits items + compat edges (BTR ammo ≠ LAV ammo).

═══ SHIPPED (do not reopen) ═══
  T-068.0.1–.6 Phase 1 flat registry + dumb loadout + NPC equip.
  T-151 wgpu map (unrelated).

═══ DO ═══
  - Expand registry-items kinds; add registry-compat.schema.json + goldens; npm run validate
  - Workbench plugin: enumerate loaded-addon prefabs; classify by rules; skip static junk
  - Derive compat edges from engine (mags/weapons/optics/vehicle weapons) — no hand tables
  - Emit items + compat to $profile:; commit sample under packages/tbd-schema/registry/
  - Retire BuildCuratedRows allowlist
  - .ai/artifacts/t150_verify_log.md + tag T-150

═══ DO NOT ═══
  - Hardcoded prefab GUID/path lists as the source of truth
  - Edit registry / CLAUDE / hub (Cursor)
  - Invent Enfusion APIs or ResourceNames
  - Implement T-068.9 ingest, T-146 browser, T-070 place, T-069 markers, Forge UI
  - Silently skip an edge family — name it in verify log

═══ VERIFY ═══
  (commands in spec §Verify — paste outputs into verify log)
  Prove: architecture includes new mods by re-run only (document procedure)

═══ RETURN ═══
  SHA + tag T-150
  Item count + edge count + edge-type histogram
  Ready for Cursor: mark T-068.8 / hub; queue T-068.9 ingest
```
