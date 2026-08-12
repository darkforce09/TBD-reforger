# T-068.15.1 — Export cargo capacity + default contents

**Ticket:** T-068 · **Slice:** T-068.15.1 · **Status:** **SHIPPED** @ `85acbb13` (tag **T-068.15.1**) ·
**Executor:** claude-code / Fable 5 ·
**Verify:** [`.ai/artifacts/t068_15_1_verify_log.md`](../../../.ai/artifacts/t068_15_1_verify_log.md) ·
**Authority:** [`t068_15_cargo_program.md`](t068_15_cargo_program.md) ·
**Spike:** [`.ai/artifacts/t068_cargo_capacity_spike.md`](../../../.ai/artifacts/t068_cargo_capacity_spike.md)

---

## In one sentence

Extend `TBD_RegistryScan` / export plugin so one Workbench export **automatically** fills
per-container `max_weight_kg`, cargo grid `W×H`, and `character_default_cargo` edges —
no hand tables.

## Problem

`TBD_RegistryScanner` already reads `m_fMaxWeight` and `MaxCumulativeVolume` off storage
components (`TBD_RegistryScan.c` `ReadPhysAttrs`) and emits them as `max_weight_kg` /
`max_volume_cm3` when serialized in the prefab. It does **not** yet export:

1. **Cargo grid dimensions** (`cargo_grid_w`, `cargo_grid_h`) for Arsenal screenshot parity.
2. **Default stored items** inside character container storages as compat edges for seeding.

Without these, T-068.15.2 cannot show capacity readouts or seed rifleman vest/pants contents.

## Goal

1. **Spike (MCP):** Pin APIs for grid dimensions and default stored items; update spike artifact
   with measured field names before schema land.
2. **Schema:** Add optional `cargo_grid_w` / `cargo_grid_h` on `registry-items` container rows;
   add `character_default_cargo` to `registry-compat` `edge_type` enum (or locked spike name).
3. **Scanner:** Always resolve container max weight when engine-readable (class default if needed);
   derive or read grid W×H per spike decision; walk character prefab storages for default cargo.
4. **Re-export + validate + import:** `make registry-import`; Class-R gates below.

## Out of scope

- Arsenal UI (**T-068.15.2**)
- Compiled flatten (**T-068.11**)
- Player equip / `InsertItem` (**T-068.12**)
- Inventing capacity numbers when prefab omits serialized values

## Locked decisions

| # | Decision |
|---|----------|
| 1 | Plugin auto-grab only — Arsenal consumes export |
| 2 | Field names locked in spike artifact before schema land |
| 3 | Never invent capacity numbers — omit when unreadable |
| 4 | Grid derivation formula must pass Class-R against ≥2 garment types before ship |
| 5 | `character_default_cargo` edge: `from_node` = stored item, `to_node` = container garment on character |

## Tasks

1. Finish spike: MCP `wb_script_editor` on `SCR_InventoryStorageBaseUI` / MenuUI resize path;
   document grid source (serialized vs derived).
2. Extend `TBD_RegistryScanItem` + `ReadPhysAttrs` (or sibling) for `cargoGridW` / `cargoGridH`.
3. Extend `DeriveEdges` for character default cargo (read storage slot contents from character prefab).
4. Extend `TBD_RegistryItemsExportPlugin.c` JSON emit for new item fields + compat edges.
5. Bump `registry-items.schema.json` + `registry-compat.schema.json`; golden samples.
6. Re-export workbench JSON; `npm run validate`; `make registry-import`.
7. `.ai/artifacts/t068_15_1_verify_log.md` + tag **T-068.15.1**.

## Verify

```bash
cargo xtask mod dev-bootstrap
bash scripts/mod/mcp-call.sh wb_connect '{}'
# wb_reload scripts → run TBD registry export
cd packages/tbd-schema && npm run validate
make registry-import
```

### Class-R gates

| ID | Check | Pass |
|----|-------|------|
| C1 | Trousers vs vest differ in `cargo_grid_w`×`cargo_grid_h` and/or `max_weight_kg` | Measured inequality in verify log |
| C2 | US rifleman character has ≥1 `character_default_cargo` edge (mag or medical) | Edge count + sample RN in log |
| C3 | No hand-authored capacity rows — all from scanner | Diff shows only plugin + re-export |

## Acceptance

- [ ] Spike artifact committed with measured APIs + locked field names
- [ ] Plugin export fills capacity + default cargo automatically
- [ ] Schema validate PASS; registry re-imported
- [ ] Class-R C1–C3 PASS
- [ ] Tag **T-068.15.1**

## Depends on / Unblocks

- **Depends on:** T-150 scanner baseline (shipped), spike artifact
- **Unblocks:** T-068.15.2 (Arsenal UI), then T-068.11 (compile cargo), T-068.12 (equip cargo)

---

## Claude Code prompt — T-068.15.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root.

Implement **T-068.15.1** — Export cargo capacity + default contents (resume WIP).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  ./scripts/ticket brief T-068
  cargo xtask mod dev-bootstrap

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t068_15_1_claude_code_handoff.md
  2. .ai/artifacts/t068_15_fable_program_handoff.md
  3. docs/specs/Mission_Creator_Architecture/t068_15_1_cargo_capacity_export.md
  4. .ai/artifacts/t068_cargo_capacity_spike.md
  5. apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryScan.c
  6. apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c
  7. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Arsenal needs per-garment max kg + grid W×H and character default cargo edges from one
  plugin export. Scanner/schema WIP exists on disk but Workbench never successfully
  recompiled/exported; API/DB lack cargo_grid columns; compat unique index collapses qty.

═══ SHIPPED (do not reopen) ═══
  T-150 universal registry export · T-068.10 Arsenal wear/weapons · spike pins for grid/cargo

═══ LOCKED ═══
  - Plugin auto-grab only — no hand capacity tables
  - Grid: VOLUME_PER_CELL=50, w=4, h=max(3,ceil(cells/4))
  - character_default_cargo: from=item, to=character, evidence TargetStorage=…; one edge per PrefabsToSpawn entry
  - Preserve cargo qty through DB ingest (fix unique-index collapse)
  - No World Editor — MCP only
  - No silent deferral of grids/defaults

═══ DO ═══
  1. Resume WIP in TBD_RegistryScan / ExportPlugin (do not rewrite from zero)
  2. Force Workbench to compile DISK scripts (restart WB if Script Editor line count ≠ disk)
  3. Run Export TBD Registry Items; land workbench JSON; make schema-validate
  4. Migration + model + import + GET /registry for cargo_grid_w/h; schema-codegen as needed
  5. Compat ingest preserves multiplicity/qty for character_default_cargo
  6. make registry-import · Class-R C1–C3 · verify log · tag T-068.15.1
  7. Continue to T-068.15.2 per program handoff (same session unless blocked)

═══ DO NOT ═══
  - Hand-author capacity rows
  - Arsenal UI / compile / player equip in this slice file set before 15.1 gates pass
  - Edit docs/** or registry.json (Cursor)
  - Use npm validate under packages/tbd-schema (use make schema-validate)
  - Rely on wb_reload ExecuteAction=false as proof of compile

═══ VERIFY ═══
  make schema-validate
  make registry-import
  Class-R: trousers ≠ vest grid and/or max kg; US rifleman ≥1 character_default_cargo; qty>1 for duplicate mags survives import
  .ai/artifacts/t068_15_1_verify_log.md

═══ RETURN ═══
  SHA + tag T-068.15.1
  Sample item JSON with cargo_grid_* + sample cargo edges
  Then start T-068.15.2 (spec prompt)
```
