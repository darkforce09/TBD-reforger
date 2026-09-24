# T-304 — RegistryScan never reads weapon weight; 32 wrong rows

Owner: command center. Operator authorization 2026-09-04: "agents may edit the Enfusion mod scripts; gate =
`cargo xtask mod compile`; in-game behaviour goes on a human checklist." Re-running the scan is the operator's step.

## Claude Code prompt — T-304

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-304 && pwd && git branch --show-current   # must be slice/T-304
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryScan.c:280-460, the tbd-export copy (diff them first), docs/plans/t-304_plan.md
═══ PROBLEM ═══
Weapon ItemPhysAttributes are skipped because the owning component name ends in StorageComponent; cross-class override order is hash order, so 32 rows carry 0.01 kg.
═══ SHIPPED ═══
T-206 probe evidence (deferred ticket); the registry JSON rows are the proof — do not edit them.
═══ LANGUAGE GATE ═══
Enforce script only.
═══ LOCKED ═══
- Both TBD_RegistryScan.c copies end byte-identical.
- No hand edits to registry data; the operator's re-scan produces the corrected rows.
═══ DO ═══
1. Paste the 32 poisoned rows and the 0/107 weight count from the registry JSON (defect on main).
2. Fix the storage split. 3. Order buckets by derivation depth. 4. Mirror to tbd-export.
5. cargo xtask mod compile ; cargo xtask platform wave gate --slice T-304
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no registry JSON edits; no new files.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask platform wave gate --slice T-304
═══ MANUAL ═══
Operator: Workbench re-scan, then diff the registry: 107/107 weapons carry weight_kg; the 32 rows show real weights.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
