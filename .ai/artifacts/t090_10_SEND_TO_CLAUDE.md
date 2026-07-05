# Send-off — T-090.8.1 (forest / rock mass render)

**CWD:** `/home/Samuel/Projects/TBD-Reforger` (`main`)

```bash
./scripts/ticket prompt T-090 --slice T-090.8.1
```

**Handoff:** [`.ai/artifacts/t090_8_1_claude_code_handoff.md`](t090_8_1_claude_code_handoff.md)  
**Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) §7 row T-090.8.1  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_8_forest_vegetation_regions.md`](../../docs/specs/Mission_Creator_Architecture/t090_8_forest_vegetation_regions.md)  
**Prior:** T-090.5.3 @ `155651b9` — [verify log](t090_5_3_verify_log.md)

**One-liner:** Green forest **polygons** from TBDD density + region hulls — still **no tree icons** (those are T-090.5.5).

**Data on disk:** `manifest.objects.densityPath` → 625 `.bin` grids · `regionsPath` → 36 regions · 501,861 trees in chunks (indexed, not rendered as glyphs).

**Single lane:** no T-090.5.4 until 8.1 ships.
