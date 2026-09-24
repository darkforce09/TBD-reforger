# T-092.0 — Spawn + compile contract (docs)

**Ticket:** T-092 · **Slice:** T-092.0  
**Status:** **shipped** (this spec) — cursor-docs  
**Executor:** cursor-docs  
**Authority:** [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)

---

## In one sentence

Freeze the three JSON artifact contract, coordinate mapping, spawn policy, and slice verification gates so T-092.1/.2 code does not guess API shapes.

---

## Problem

Editor compile emits `editor.slots` only; mod calls wrong API path; no optional `y` or `headingDeg`; slot IDs are UUIDs incompatible with golden missions.

---

## Goal

1. Hub [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md) — artifact diagram, gaps table.
2. Slice specs: [`t092_1_mod_spawn_policy.md`](t092_1_mod_spawn_policy.md), [`t092_2_mod_compile_route.md`](t092_2_mod_compile_route.md).
3. Document verified repo gaps (compile.ts, TBD_MissionLoader path, bridgehead golden).

---

## Verification gate

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | Hub on disk | `t092_spawn_transform_program.md` exists |
| S2 | Slice specs | `t092_1_*`, `t092_2_*` linked from hub |
| S3 | Registry | T-092 slices in registry.json |
| S4 | No T-096 confusion | Spawn transform = **T-092** only in docs |

```bash
test -f docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md
test -f docs/specs/Mission_Creator_Architecture/t092_1_mod_spawn_policy.md
test -f docs/specs/Mission_Creator_Architecture/t092_2_mod_compile_route.md
./scripts/ticket check --strict
```

---

## Related

- [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)
- [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md)
