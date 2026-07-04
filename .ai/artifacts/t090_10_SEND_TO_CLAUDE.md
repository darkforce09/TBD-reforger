# Send-off — T-090.3.2 (density grids + PH-P2 trees)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

```bash
./scripts/ticket prompt T-090 --slice T-090.3.2
```

**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §3.3 + §7 row T-090.3.2  
**Prior:** T-090.3.1 shipped @ `e47f25fc` — [verify log](t090_3_1_verify_log.md)

**Scope:** `objects/density/{cx}_{cy}.bin` (TBDD 32 m corners) + PH-P2 tree instance chunks + forest-regions (t090_8 path B) + census real ints. Re-export from staged raw JSONL — **no second full Workbench run** if raw still staged.

**Single lane:** no T-090.5.1 until this ships.

**Workbench note:** new plugin classes require **Script Editor compile** — `wb_reload` alone does not register them.
