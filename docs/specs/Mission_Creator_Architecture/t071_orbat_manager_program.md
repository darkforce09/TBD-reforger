# T-071 — ORBAT Manager (web program)

**Status:** **READY** (T-071.1+) — **T-071.0 SHIPPED** via **T-177** @ `e97a01c6` (tag **T-177**).  
**Ticket:** T-071 · **Route:** `/missions/:id/edit` · **Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)  
**Authority:** [MC ROADMAP](ROADMAP.md) · [`eden/gap_analysis.md`](eden/gap_analysis.md) · [`ux_spec.md`](ux_spec.md)

**T-071.0 (done):** left Outliner → Editor Layers only; top-strip **ORBAT Manager** → `OrbatManagerDialog` browse/select shell (select + dbl-click→Attributes). Spec/verify: [`t177_mc_chrome_orbat_cutover.md`](../../platform/t177_mc_chrome_orbat_cutover.md) · [`.ai/artifacts/t177_verify_log.md`](../../../.ai/artifacts/t177_verify_log.md).

**Next:** **T-071.1** squad CRUD · **T-071.2** slot numbering/export order · **T-071.3–.4** logos/standardizations/per-slot Arsenal.

**Map gate:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md) — **cleared**.

---

## Program order (locked)

```text
T-090 / T-091 / T-092 ✓ @ `a73224f2` → **T-071.0–.2** (active) → T-068.13 → T-068.7+
```

Phase 1 Virtual Arsenal (test NPC only) stays shipped. **Do not** resume T-068 Phase 2 loadout until ORBAT baseline + mod slot picker exist.

---

## Three workstreams (same mission JSON)

| Workstream | Where | Ticket | Role |
|------------|-------|--------|------|
| **Author ORBAT** | Website Mission Creator | **T-071** | Squad names, membership, slotting order — **writes** export truth |
| **Play / slotting** | Arma Reforger mod | **T-068.13** → **T-114** | Production LOBBY slot picker — **reads** compiled `slots[]` |
| **Map / spawn accuracy** | Website + mod | **T-090–T-092** ✓ | DEM, tiles, mod native compile, spawn Y/yaw — **gate cleared** @ `a73224f2` |

**Map/spawn gate cleared @ `a73224f2`.** ORBAT/loadout Phase 2 (T-068.7+) waits on **T-071.2** + **T-068.13**.

---

## Minimum bar before T-068 Phase 2 loadout

1. **T-092.2** ✓ @ `a73224f2` — mod compiled `slots[]` + spawn Y/yaw verified (verify @ `452ce501`)
2. **T-071.2** — squad order in export
3. **T-068.13** — production LOBBY slot picker (Aegis UX; no CLI)
4. **T-068.11–.14** — loadout on **human player**

---

## Honest status (2026-07-19)

**T-071.0 browse shell is live** (T-177). Full Eden ORBAT authoring (CRUD / export order / logos) is still **T-071.1+**. Event attach/inline claim (T-008–T-010) ≠ that remaining work.

| Surface | Exists | Missing |
|---------|--------|---------|
| Mission Creator | Top-strip **ORBAT Manager** modal (browse/select/Attributes); left = Editor Layers only | Squad CRUD, numbering, export order, logos, standardizations (**T-071.1+**) |
| Event Hub | Auto-materialize ORBAT; inline claim | Full slotting-screen parity; **T-118** admin polish |
| Compiler / mod | `orbat[]` + editor graph; **GET /compiled** + flatten @ T-092 | Slot order, loadout blocks (T-068.11+) |

---

## Slice ladder

| Slice | Focus | Status |
|-------|-------|--------|
| **T-071.0** | Modal shell; left sidebar → Editor Layers only | ✅ via **T-177** @ `e97a01c6` |
| **T-071.1** | Squad CRUD; move slot between squads | next |
| **T-071.2** | Slot numbering + slotting order in export | queued |
| **T-071.3–.4** | Logos, standardizations, per-slot Arsenal link | queued |

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
