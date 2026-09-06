# Wave 247 adversarial verify

HEAD at verify start: `3bfc58b9d` (UNREAD retirement). Base `05ee3f3a6`. Merges T-681 `b173c7c68`, T-936.3 `b82424167`, T-133 `c58cf917c`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. `:3000` / `:8080` left listening.

**In-wave BLOCKER fix (schema wording, not a slice):** `$defs/task` had `additionalProperties: false` and no `schedule`. T-936.3 restored `tasks[]` on flatten (`authored_blocks_root` clones the authored Value), so a scheduled task 500'd `/compiled`. Proved: `cargo xtask schema validate-file /tmp/t133-scheduled-task.json` → `/tasks/0 Additional properties are not allowed ('schedule' was unexpected)`. After adding `$defs/task.schedule` + `$defs/taskSchedule`: same file **ok**; unscheduled golden still **ok**; `windowS: 0` → `/tasks/0/schedule/windowS 0 is less than or equal to the minimum of 0`. `cargo xtask schema validate` → All contracts valid.

## Findings

### 1. MAJOR — flatten still drops params / group AI / vehicle state (T-946.36)

`GetRawJson()` is the `/compiled` body. `authored_blocks_root` now copies `winConditions`, `tasks`, `radioPlan` — not `missionParams`. `ModOrbatGroup` still has no combatMode/behaviour/formation/speedMode. `ModVehicle` still has no lock/fuel/ammo. Entity health/allowDamage/showModel/size/stamina likewise never leave flatten (T-681 readers need a hand-staged document). Already filed T-946.36; not re-filed.

### 2. MAJOR — tasks and radio panels never mount (T-946.33 + T-946.39)

`settings_modal.rs` still only mounts `{win_conditions_card(ctrl)}`. `panels/mod.rs` publishes `tasks_panel` and `radio_panel`. Operator cannot author tasks or radioPlan in the UI. T-946.33 covers tasks; T-946.39 files radio (same file).

### 3. MAJOR — entity bool false cannot hide or invuln; stamina skip-logged (T-946.40)

`TBD_EntityState.c` applies `allowDamage` / `showModel` only when bound true (T-946.37 class). Authored false equals omit. Schema types `stamina` as boolean; Reforger has no per-character enable (`BaseStaminaComponent.GetStamina` only). Authored `true` logs once and is skipped.

---

## Attacked and FAILED to break

- **Gate vacuity / unexamined code (BLOCKER class):** `tbd_wave247_cold` exists; `missions_it` is 35 tables / 38 missions / 22 migrations. Wave gate at UNREAD `3bfc58b9d` base `05ee3f3a6`: **GATE: PASS** (`/tmp/wave247-gate.txt`). Wave-level `cargo xtask mod compile`: OK, 5750 files / **11397** classes (wave 246 was 11388). Gate still does not run `mod compile` (T-946.17). Not re-filed.
- **Schedule × flatten 500:** was a BLOCKER; fixed this pass (schema only). T-133 `KNOWN_KEYS` already accepted `schedule`; the hole was `$defs/task`.
- **T-946.35 flatten drops tasks[]:** **fixed by T-936.3** (`authored_blocks_root` inserts `tasks`). Ticket cancelled. Do not re-file.
- **Export twins / T-946.26:** `TBD_TaskStateMachine.c` and `TBD_EntityState.c` are byte-identical ASCII (`cmp -s`). `TBD_MissionLoader.c` / `TBD_SpawnManager.c` still differ emdash vs hyphen in comments only (pre-existing).
- **UNREAD tripwires:** `allowDamage` / `showModel` / `stamina` / `health` retired. `size` expected **14** (T-673 marker.size + T-681 SetScale; `why` contains `different`). `shape` expected 34. Fire-once still `placementRadius`. `cargo xtask schema validate` → All contracts valid.
- **Derived radio Class-R:** T-936.3 used `DOCUMENT_OWNED` for `radioPlan`; default derive path stays byte-identical when nothing is authored (slice report). Not re-broken here.
- **Slices edited schema_gates.rs / mission.schema.json:** merge commits do not. CC retired UNREAD after land; CC added `schedule` after the flatten×schema attack.

NIT: `tasks.rs` still comments that `$defs/task` does not declare `schedule` — stale after this schema insert.

## main_left_clean

- tracked dirty at verify start: `packages/tbd-schema/schema/mission.schema.json` (the BLOCKER fix)
- `git rev-parse HEAD` at gate: `3bfc58b9d`
- `:3000` and `:8080` still LISTEN
