# T-068.8 — Workbench compat matrix export

**Ticket:** T-068 · **Slice:** T-068.8  
**Status:** Spec ready — Phase 2  
**Executor:** claude-code (**enfusion-mcp required**)  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Workbench exports compat edges JSON for ingest into `registry_compat` (Phase 2).

---

## Problem

Smart Forge needs attachment/mag/ammo compatibility data from game assets.

---

## Goal

1. Export script emits compat graph per T-068.7 finalized spec.
2. Validate against `registry-compat.schema.json` (after T-068.9 creates schema — re-validate then).
3. Sample export committed under `packages/tbd-schema/registry/` or mod Data.

---

## Out of scope

- Website ingest (T-068.9)
- UI (T-068.10)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Executor | **claude-code** + enfusion-mcp (same contract as T-068.1) |
| Node IDs | Full ResourceName |

---

## Tasks

1. Workbench export tooling (human)
2. Sample compat export file + stats (edge count)

---

## Verify

```bash
cd packages/tbd-schema && npm run validate
EXPORT=registry/registry-compat.workbench.json
test -f "$EXPORT"
jq -e '.edges | length >= 50' "$EXPORT"
jq -e '.edges[0].edge_type != null' "$EXPORT"
```

---

## Verification gate (mandatory)

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Export exists | File at documented path |
| A2 | Schema | Validates once `registry-compat.schema.json` exists (T-068.9); until then: jq structural + T-068.7 edge_type enum match |
| A3 | Volume | ≥50 compat edges |
| A4 | Node IDs | Sample 3 edges: `from`/`to` match ResourceName regex |
| A5 | Taxonomy | Every `edge_type` in export listed in T-068.7 doc |

Paste: validate output + `jq '.edges | group_by(.edge_type) | map({type: .[0].edge_type, count: length})'`

---

## Depends on / Unblocks

- **Depends on:** T-068.7
- **Unblocks:** T-068.9

---

## Workbench checklist

- [ ] Export runs
- [ ] Edge types match T-068.7 taxonomy
- [ ] Paste: path + edge count
