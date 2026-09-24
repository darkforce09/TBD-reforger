# T-290 — Nine dead flatten fields the mod never reads

Owner: command center. Frozen-scope audit ticket; proposed scope engine/core; pack_last on flatten.rs.
Operator authorization 2026-09-04 covers the two .c readers (gate = `cargo xtask mod compile`).

## Claude Code prompt — T-290

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-290 && pwd && git branch --show-current   # must be slice/T-290
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
crates/map-engine-core/src/mission/flatten.rs:1700-1900 and :2200-2300, Backend/TBD_MissionLoader.c, Backend/TBD_MissionValidator.c, docs/plans/t-290_plan.md
═══ PROBLEM ═══
Nine emitted fields have no reader and no record of being unread.
═══ SHIPPED ═══
T-674.1/T-675.1/T-936.1 flatten changes (land first) — the ledger must include their fields too.
═══ LANGUAGE GATE ═══
Rust + Enforce script. No scripts.
═══ LOCKED ═══
- Ledger test enforces coverage of every top-level emitted key; no field is dropped from the wire.
- briefingSeconds stays advisory (AdminSetSeconds contract unchanged).
═══ DO ═══
1. Ledger + test; paste the red on main. 2. Three readers + validator warning; mod compile. 3. Annotations.
4. Perturb (delete a row) → red → restore → touch → green. 5. Wave gate.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no schema edits; no removal of emitted fields.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features mission::flatten ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-290
═══ MANUAL ═══
Human checklist: load a compiled mission; the loader log shows author, templateId and mode.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
