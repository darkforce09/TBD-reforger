# T-302 — Prove T-182 weapon equip on a live body

Owner: command center. Operator authorization 2026-09-04: "agents may edit the Enfusion mod scripts; gate =
`cargo xtask mod compile`; in-game behaviour goes on a human checklist."

## Claude Code prompt — T-302

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-302 && pwd && git branch --show-current   # must be slice/T-302
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c (header :1-110 + weapon phase), xtask/src/mod_world_boot.rs, docs/plans/t-302_plan.md
═══ PROBLEM ═══
Weapon equip compiles but has never been observed; no log line says which slot got which weapon.
═══ SHIPPED ═══
T-182 equip phases (insert/replace fallback, worn verify) — do not change the equip logic.
═══ LANGUAGE GATE ═══
Enforce script + Rust only. No shell scripts.
═══ LOCKED ═══
- Log format `[TBD][Equip] slot=<n> weapon=<res> result=<ok|replaced|failed>` exactly; xtask greps it.
- Fixture loadout has four distinct weapons; assertion = four ok, zero replaced.
═══ DO ═══
1. Add the log line. 2. Add the fixture. 3. Extend mod_world_boot.rs assertion.
4. Perturb (expect five lines) → red → restore → touch → green. 5. mod compile + wave gate.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no WeaponSlotComponent.SetWeapon; touch only the two owned files.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-302
═══ MANUAL ═══
Human checklist: boot the fixture mission, take the four-weapon slot, confirm all four weapons carried and none replaced.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
