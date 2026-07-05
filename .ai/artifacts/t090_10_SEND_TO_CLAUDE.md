# Send-off — T-090.5.3 (chunk streaming @ scale)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §7 row T-090.5.3  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md`](../../docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md)  
**Worker:** [`docs/specs/Mission_Creator_Architecture/t090_world_objects_worker.md`](../../docs/specs/Mission_Creator_Architecture/t090_world_objects_worker.md) (if present)  
**Prior:** T-090.5.2.2 @ `346a31c9` · T-090.3.3 @ `887a6ed1` — [verify log](t090_5_2_verify_log.md)

**Scope:**

- Full `worldObjects.worker.ts` (W1–W3, W5 + W4-v2 `visibleInstances`)
- `chunkStore.ts` LRU/budget/border/oversized on main thread
- Replace interim `worldData.ts` main-thread fetch with worker hydration
- `≤4 ms/frame` apply budget; N11 PH-P1/P2 budgets
- Shrink `worldData.ts` to manifest gate only

**Gates:** W1–W5(v2); INSTANCE_BUDGET vitest; hydrate timing; ≥55 fps @ P1+P2 data with flag on.

**Single lane:** no T-090.8.1 until 5.3 ships (plan order: 5.3 → 5.2 already done → 8.1).

**Do not rewrite:** `roadLayer`, `buildingLayer`, `styleModes`, `lodGates`, export pipeline.

**Operator:** `VITE_WORLDMAP_ENABLED=1` + hard refresh after deploy (module caches data).
