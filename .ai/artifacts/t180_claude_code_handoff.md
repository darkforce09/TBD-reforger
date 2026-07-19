# T-180 — Claude Code / Grok handoff (ORBAT + Eden placement)

**Hub:** [`docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md`](../../docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md)  
**Pins:** [`docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md`](../../docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md)  
**Active:** **T-180.2** — [`t180_2_graph_mutators.md`](../../docs/specs/Mission_Creator_Architecture/t180_2_graph_mutators.md)  
**CWD:** repo root · **Branch:** `main`  
**Stitch:** [`.ai/artifacts/t180_stitch_orbat_modal/`](t180_stitch_orbat_modal/)

## Shipped

| Slice | SHA / tag | Notes |
|-------|-----------|-------|
| **T-180.1** | `aeb51209` / **T-180.1** | `place_character_under_side`, `set_leader`, callsign/rank, `active_side`, `FactionRow.key`. Verify: [`t180_1_verify_log.md`](t180_1_verify_log.md). Core tests: `--features doc`. |

## Simple version

Next: **T-180.2** — graph mutators + empty-squad GC + `add_vehicle` (see B-L8). Copy prompt from `t180_2_graph_mutators.md`.

## Critical pins

| Pin | Detail |
|-----|--------|
| Packages | `map-engine-core` · `website-frontend` · `website-api` |
| Core tests | Always `--features doc` for MissionDocCore |
| Place | Already mints new squad under side — mutators must preserve invariants |
| Vehicles | `add_vehicle` still ABSENT — add in .2 |
| Colors / Arsenal tab | See pins (.3 / .9) |

## Slice order

```text
.1 ✓ → .2 (NOW) → .3 → .4 → .5 → .6 → .7 → .8 → .9
```

## Do not

Docs/registry · Stitch UI · skip GC · reuse `move_slot_to_layer` as squad refile

## Return

SHA + tag `T-180.2` · `.ai/artifacts/t180_2_verify_log.md` · Ready for Cursor sync / T-180.3
