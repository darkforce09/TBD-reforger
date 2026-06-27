# T-068.7 — Compat matrix spec (Phase 2 docs)

**Ticket:** T-068 · **Slice:** T-068.7  
**Status:** Spec ready — **paused** until **T-090–T-092** + **T-071.2** gates clear  
**Executor:** cursor-docs  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Finalize Phase 2 compat matrix documentation — edge types, Postgres shape, worker contract — no code.

---

## Problem

Phase 2 needs a written contract before Workbench compat export and worker ingest.

---

## Goal

1. Expand this file (Cursor) with: edge types (`optic_on_weapon`, `mag_in_weapon`, `ammo_in_mag`, `vest_slot`, …).
2. Postgres `registry_compat` table shape (from_node, to_node, edge_type, modpack_id).
3. Worker API sketch: `canEquip`, `canAttach` inputs/outputs.
4. **`registry-compat.schema.json` deferred to T-068.9** (Claude Code implements file).

---

## Out of scope

- Implementing schemas or worker (T-068.8–T-068.9)
- Starting before map gate (**T-090–T-092**) + **T-071.2**

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Graph identity | Nodes = `resource_name` strings |
| Phase 2 gate | Human approval after T-068.6 |

---

## Tasks (Cursor on execution)

1. Finalize edge taxonomy table in this doc
2. Add Postgres DDL sketch section
3. Link from program hub Phase 2 section

---

## Verify

```bash
./scripts/ticket sync && make ticket-check-strict
grep -E 'edge_type|registry_compat|canEquip' docs/specs/Mission_Creator_Architecture/t068_7_compat_matrix_spec.md
```

---

## Verification gate (mandatory)

**Advance only after T-068.6 PASS sign-off with Phase 2 approved: YES.**

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Phase 1 gate | T-068.6 verify paste exists with PASS |
| A2 | Edge taxonomy | ≥4 `edge_type` rows documented with definitions |
| A3 | Postgres sketch | DDL block for `registry_compat` present |
| A4 | Worker contract | `canEquip` / `canAttach` inputs/outputs documented |
| A5 | Ticket sync | `make ticket-check-strict` exit 0 |
| A6 | Human approval | Paste contains literal **Phase 2 approved: YES** |

---

## Depends on / Unblocks

- **Depends on:** T-068.6 pass + approval
- **Unblocks:** T-068.8

---

## Documentation sync (Cursor)

Update `engineering_plan.md` §Registry worker cross-ref when finalized.

---

## Stub — edge types (expand on T-068.7 execution)

| edge_type | Meaning |
|-----------|---------|
| `weapon_optic` | Optic attaches to weapon rail |
| `weapon_mag` | Mag fits weapon |
| `mag_ammo` | Ammo fills mag |
| `uniform_vest` | Vest over uniform slot |

Postgres DDL and export envelope details to be completed when slice executes.
