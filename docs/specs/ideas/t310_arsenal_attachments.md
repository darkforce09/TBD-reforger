# T-310 — Arsenal attachments never reach the compiled document

Owner: command center. Operator authorization 2026-09-04 covers the equip helper edit (gate = `cargo xtask mod compile`).

## Claude Code prompt — T-310

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-310 && pwd && git branch --show-current   # must be slice/T-310
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
flatten.rs:1480-1560, mission.schema.json:370-410 (gear), Gamemode/TBD_LoadoutEquipHelper.c:1-120 + weapon phase, docs/plans/t-310_plan.md
═══ PROBLEM ═══
Attachments are authored and weighed but never compiled or mounted.
═══ SHIPPED ═══
T-197 Arsenal edges, T-182 equip phases, T-302 equip log lines (land first — extend the same log format).
═══ LANGUAGE GATE ═══
Rust + JSON schema + Enforce script. No scripts.
═══ LOCKED ═══
- gear.attachments optional; loadouts without it compile byte-identically.
- Mount failures log and continue; never block a spawn.
- generated/ via schema-codegen only.
═══ DO ═══
1. Paste the compiled gear missing the suppressor. 2. Schema + golden. 3. Emit. 4. Mount + log. 5. Perturb (skip emit) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no UI edits; no WeaponSlotComponent.SetWeapon.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features mission::flatten ; cargo xtask ci schema-validate ; cargo xtask ci schema-codegen ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-310
═══ MANUAL ═══
Human checklist: pick a suppressor in the Arsenal, spawn, inspect the rifle.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
