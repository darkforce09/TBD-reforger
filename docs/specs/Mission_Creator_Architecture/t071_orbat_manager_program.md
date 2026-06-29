# T-071 — ORBAT Manager (web program)

**Status:** **QUEUED** — blocked on **T-092.2** (map/spawn verify).  
**Ticket:** T-071 · **Route:** `/missions/:id/edit` · **Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)  
**Authority:** [MC ROADMAP](ROADMAP.md) · [`eden/gap_analysis.md`](eden/gap_analysis.md) · [`ux_spec.md`](ux_spec.md)

**Map gate:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)

---

## Program order (locked)

```text
T-090 / T-091 / T-092 (map + mod compile) → T-071.0–.2 → T-068.13 → T-068.7+
```

Phase 1 Virtual Arsenal (test NPC only) stays shipped. **Do not** resume T-068 Phase 2 loadout until ORBAT baseline + mod slot picker exist.

---

## Three workstreams (same mission JSON)

| Workstream | Where | Ticket | Role |
|------------|-------|--------|------|
| **Author ORBAT** | Website Mission Creator | **T-071** | Squad names, membership, slotting order — **writes** export truth |
| **Play / slotting** | Arma Reforger mod | **T-068.13** → **T-114** | Production LOBBY slot picker — **reads** compiled `slots[]` |
| **Map / spawn accuracy** | Website + mod | **T-090–T-092** | **Hard gate** — DEM, tiles, mod native compile, spawn Y/yaw |

**Map/spawn workstream must PASS before ORBAT/loadout Phase 2 (T-068.7+) is meaningful.**

---

## Minimum bar before T-068 Phase 2 loadout

1. **T-091.0 verify PASS + T-092.2** — anchor verify done @ `6d96339`; mod compiled `slots[]` + spawn Y/yaw still pending
2. **T-071.2** — squad order in export
3. **T-068.13** — production LOBBY slot picker (Aegis UX; no CLI)
4. **T-068.11–.14** — loadout on **human player**

---

## Honest status (2026-06)

**Web ORBAT is not built.** Event attach/inline claim (T-008–T-010) ≠ Eden ORBAT Manager.

| Surface | Exists | Missing |
|---------|--------|---------|
| Mission Creator | Read-only Faction → Squad → Slot tree; default squads on drop | Squad names, numbering, order, ORBAT Manager modal, logos, standardizations |
| Event Hub | Auto-materialize ORBAT; inline claim | Full slotting-screen parity; **T-118** admin polish |
| Compiler / mod | `orbat[]` + editor graph; **no mod flatten until T-092** | Slot order, loadout blocks (T-068.11+) |

---

## Slice ladder

| Slice | Focus |
|-------|-------|
| **T-071.0** | Modal shell; left sidebar → Editor Layers only |
| **T-071.1** | Squad CRUD; move slot between squads |
| **T-071.2** | Slot numbering + slotting order in export |
| **T-071.3–.4** | Logos, standardizations, per-slot Arsenal link |

Executor: **claude-code**. Docs: **cursor-docs** when activated.

---

## Related

| Ticket | Role |
|--------|------|
| **T-068.11–T-068.14** | Loadout Phase 2 + production slot picker + E2E |
| **T-118** | Event admin ORBAT + identity linking |
| **T-114** | Roster-synced mod picker (after T-068.13 + T-118) |

---

## Acceptance north star

Mission maker configures squads once in ORBAT Manager → Save → Event attach and mod `slots[]` show the **same** structure players see when slotting — without hand-editing JSON.
