# T-068.0.1 — Registry JSON schemas + golden fixtures

**Ticket:** T-068 · **Slice:** T-068.0.1  
**Status:** **shipped** @ `2487d59` (T-068.0.1)  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

**Agent roles (locked):** **Cursor** authors this spec. **Claude Code** implements schema files only.

---

## In one sentence

Define and validate `registry-items` and `loadout-export` JSON Schema files plus golden fixtures in `packages/tbd-schema/`.

---

## Problem

No shared contract for Workbench flat export, API seed/import, dumb loadout download, or mod equip test — alias POC [`registry.schema.json`](../../../packages/tbd-schema/schema/registry.schema.json) is a separate mod-spawn layer.

---

## Goal

1. Add `packages/tbd-schema/schema/registry-items.schema.json` — envelope `{ registryItemsVersion, modpackId, generatedAt, items[] }`.
2. Each item: `resource_name`, `display_name`, `category` (slash path), `icon_url` (optional), `kind` enum (`character` | `gear_primary` | `gear_uniform` | `gear_vest` | `gear_helmet`).
3. Add `packages/tbd-schema/schema/loadout-export.schema.json` — `{ loadoutVersion: "1", modpackId, gear: { primary, uniform, vest, helmet } }` (each gear value string or null).
4. Golden fixtures under `packages/tbd-schema/registry/` (e.g. `registry-items.sample.json`, `loadout-export.sample.json`).
5. Wire `npm run validate` to validate new schemas + fixtures (extend `scripts/validate.mjs` if needed).
6. Document coexistence: alias `registry.schema.json` unchanged; mod `Data/registry.json` unchanged.

---

## Out of scope

- Go/React code
- `registry-compat.schema.json` (Phase 2 / T-068.9)
- Map/topo schemas (T-090/T-091/T-110)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Identity field | `resource_name` — full `{GUID}Prefabs/.../File.et` |
| Alias registry | Separate; not merged into `registry-items` |
| Phase 1 gear kinds | Four gear kinds + `character` only |
| loadoutVersion | `"1"` string |

---

## Tasks

1. `packages/tbd-schema/schema/registry-items.schema.json` — new
2. `packages/tbd-schema/schema/loadout-export.schema.json` — new
3. `packages/tbd-schema/registry/registry-items.sample.json` — golden
4. `packages/tbd-schema/registry/loadout-export.sample.json` — golden
5. `packages/tbd-schema/scripts/validate.mjs` — include new files
6. `packages/tbd-schema/README.md` — brief note on two registry layers

---

## Verify

```bash
cd packages/tbd-schema && npm run validate
```

---

## Verification gate (mandatory)

**Advance `T-068.0.1` → `T-068.1` only when ALL PASS.**

### Automated (exit 0)

```bash
cd packages/tbd-schema && npm run validate
test -f schema/registry-items.schema.json
test -f schema/loadout-export.schema.json
test -f registry/registry-items.sample.json
test -f registry/loadout-export.sample.json
# GUID prefix on every sample item resource_name
jq -e '.items | length >= 5' registry/registry-items.sample.json
jq -e '[.items[].resource_name | test("^\\{[0-9A-F]{16}\\}")] | all' registry/registry-items.sample.json
jq -e '.loadoutVersion == "1"' registry/loadout-export.sample.json
jq -e '[.gear.primary, .gear.uniform] | any(. != null)' registry/loadout-export.sample.json
# Alias POC unchanged — still validates
jq -e '.entries | length >= 1' registry/registry.example.json
```

`npm run validate` must print `All contracts valid.` with **zero** `FAIL` lines.

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Schema files | Both new `.schema.json` files exist |
| A2 | Golden fixtures | Both sample JSON files exist and validate |
| A3 | ResourceName shape | 100% of `items[].resource_name` match `^\{[0-9A-F]{16}\}` |
| A4 | Kind enum | Sample includes at least one `character` and one `gear_*` kind |
| A5 | loadout-export | `loadoutVersion` is `"1"`; `gear` object has four keys |
| A6 | Alias coexistence | Existing `registry.schema.json` + golden alias files still in `npm run validate` |
| A7 | validate.mjs | New fixtures wired into `scripts/validate.mjs` (not orphan files) |

### Verify paste (required)

Paste **Verify template** from program hub with A1–A7 table + full `npm run validate` output.

### Manual (spot-check)

- Open `registry-items.sample.json` — no alias strings (`kit:`) in `resource_name` fields.

---

## Depends on / Unblocks

- **Depends on:** T-068.0 (BUILD)
- **Unblocks:** T-068.1, T-068.2, T-068.4, T-068.5

---

## Documentation sync (Cursor)

After merge: none (schema-only slice); program hub unchanged.

---

## Claude Code prompt — T-068.0.1

```
Read CLAUDE.md §Status. Active slice: T-068.0.1.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_0_1_registry_schemas.md
Do not edit documentation. Branch: ticket/T-068
Deliver: packages/tbd-schema/schema/registry-items.schema.json,
         loadout-export.schema.json, golden fixtures, validate script updates.
Verify: cd packages/tbd-schema && npm run validate (all gate commands in spec §Verification gate)
Return: Verify paste block (A1–A7 PASS table) + full command output + files changed list.
```
