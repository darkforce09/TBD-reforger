# T-068.1 — Workbench flat ResourceName export

**Ticket:** T-068 · **Slice:** T-068.1  
**Status:** Spec ready — code pending  
**Executor:** workbench  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Export flat `registry-items` JSON from Workbench (characters + Phase 1 gear) validated against T-068.0.1 schema.

---

## Problem

API and palette need real Enfusion ResourceNames; mock catalog uses fake ids.

---

## Goal

1. Extend mod export script (Workbench / Enfusion) to emit `registry-items` envelope per schema.
2. Categories: `NATO/Men/...`, `NATO/Gear/Primary/...`, etc. — slash paths for frontend tree.
3. Include ≥20 rows: US rifleman/SL/medic + sample primary/uniform/vest/helmet ResourceNames.
4. Output path documented: e.g. `apps/mod/tbd-framework/Data/registry-items.export.json` or repo `packages/tbd-schema/registry/registry-items.workbench.json`.
5. Validate export via package validate (T-068.0.1 wires export into `npm run validate`) **or** dedicated script documented in T-068.0.1 README.

---

## Out of scope

- Postgres ingest (T-068.2 import cmd)
- Compat edges (T-068.8)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Format | `registry-items.schema.json` envelope |
| MCP | Use `apps/mod/.cursor/mcp.json` when researching prefab paths |
| Vanilla modset | Arma Reforger base + TBD framework |

---

## Tasks

1. Workbench script / export component in `apps/mod/tbd-framework/` (human implements)
2. Sample export JSON committed or attached to verify paste
3. Validation log from `validate-file.mjs`

---

## Verify

```bash
cd packages/tbd-schema && npm run validate
# After export committed (adjust path if different):
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

**Advance when ALL PASS** (may run parallel to T-068.2 after T-068.0.1; advance pointer only when this slice verifies).

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Export file exists | Committed path documented in paste |
| A2 | Schema valid | `npm run validate` exit 0 including workbench export |
| A3 | Row count | `items.length >= 20` |
| A4 | Kinds coverage | ≥5 `character`, ≥1 each gear kind |
| A5 | ResourceName | 100% items match GUID ResourceName regex |
| A6 | Categories | ≥3 distinct `category` paths with `/` segments |
| A7 | Workbench spot-check | Paste 3 `resource_name` values copied from Workbench "Copy Resource Name" matching export rows |

### Verify paste (required)

Program hub template + **A7** triple match (Workbench copy vs JSON field).

---

## Depends on / Unblocks

- **Depends on:** T-068.0.1
- **Unblocks:** T-068.6 (real data); optional import into DB before E2E

---

## Documentation sync (Cursor)

After verify paste: note export path in T-068.6 checklist if changed.

---

## Workbench checklist (human)

Use **§Verification gate** acceptance table A1–A7 — every row PASS before paste.

---
