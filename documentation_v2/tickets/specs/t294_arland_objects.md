# T-294 — Arland has a manifest and no object data

Owner: command center. MANUAL step = the operator's Workbench export, runbook copied from
xtask/src/gate_export_terrain.rs:58-85.

## Claude Code prompt — T-294

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-294 && pwd && git branch --show-current   # must be slice/T-294
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
xtask/src/gate_export_terrain.rs (all), packages/map-assets/terrain-registry.json, packages/map-assets/everon/manifest.json (shape), docs/plans/t-294_plan.md
═══ PROBLEM ═══
arland is registered but has no object data; the gate has only run for everon.
═══ SHIPPED ═══
everon P1-P5 via the same gate; T-935.13 cutover changes the emit format later — P1 here uses today's pipeline.
═══ LANGUAGE GATE ═══
Rust + JSON only. No shell scripts.
═══ LOCKED ═══
- The agent never runs Workbench; if staging/export/raw-entities.jsonl for arland is missing, stop and print the runbook (that IS the MANUAL gate).
- LFS rules (.gitattributes:3-6) unchanged.
═══ DO ═══
1. Confirm arland has manifest only (paste `ls`). 2. Wait for the staged export (operator). 3. Run the gate; fix everon-only assumptions.
4. Fill manifest.json. 5. Perturb (missing staging dir) → refusal → restore → touch → green. 6. Gate.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no edits to everon assets or tools/.
═══ VERIFY ═══
cargo xtask map export-terrain arland --phase P1 ; cargo xtask ci verify terrain-manifest ; cargo xtask platform wave gate --slice T-294
═══ MANUAL ═══
Operator: 1. Workbench: open arland with all layers (wb_state ~1M+ entities). 2. Plugins > TBD > "Export TBD World Objects (full)" (or MCP wb_execute_action with menuPath "Plugins,TBD,Export TBD World Objects (full)"). 3. `world copy-export-profile --terrain arland --full --profile "$PROFILE_DIR"`. 4. Re-run the gate.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
