# Wave 246 adversarial verify

HEAD confirmed: `d51a04518` (UNREAD retirement `9bf1d6773` + edition-2024 rustfmt). Base `f09955da0`. Merges T-678 `2bfbbebc8`, T-680 `24117ac36`, T-684 `7630c6fe6`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. No application fixes this pass. Porcelain empty. `:3000` / `:8080` left listening.

## Findings

### 1. MAJOR — flatten still drops the three readers' wire keys

`GetRawJson()` is the `/compiled` body (`TBD_MissionLoader.c:368-377`, `:530`). Editor flatten is what a saved mission actually serves.

- `flatten.rs` has **zero** `missionParams` / `combatMode` hits.
- `ModOrbatGroup` (`:292-313`) is callsign / type / roles / leaderSlotId only — no combatMode, behaviour, formation, speedMode.
- `ModVehicle` (`:86-121`) is alias / uid / x / z / headingDeg / faction / seats / inventory — no lock, fuel, ammo.
- `EditorPayload::authored_blocks_root` (`:1404-1410`) still copies **only** `winConditions` (T-946.35 class).

T-678 / T-680 second-parse `GetRawJson()` and T-684's `TBD_MissionDocumentStruct.missionParams` therefore see empty authored values on any mission that went through `/compiled`. Hand-staged 1.3 JSON / golden `schema-1_3-wire-fields.json` still reach the readers (slice reports). Same hole T-946.35 already named for `tasks[]`. T-936.3 owns `flatten.rs`.

**Fix:** emit `missionParams[]` on the compiled root; add the four group fields on `ModOrbatGroup`; add lock/fuel/ammo on `ModVehicle` (and the entity twin if that path is the source). Add flatten tests that would have failed a `/compiled` probe. Do not land here — T-936.3 already owns the file.

### 2. MAJOR — authored `lock: false` cannot unlock a locked prefab

`TBD_VehicleStateWireStruct.lock` is a bool. `JsonLoadContext` binds omit and `false` identically (T-676 `repeat`). `Apply` calls `LockPilotControls(true)` only when the bound value is true (`TBD_VehicleState.c:18-20`, `:154-155`, `:237`). A prefab that spawns with pilot controls locked stays locked when the author writes `"lock": false`.

**Fix:** presence-aware bool (sentinel / optional), or apply `LockPilotControls(false)` when the key is present and false.

### 3. MAJOR — vehicle state is `vehicles[]`-only

The second parse declares `vehicles[]` (`TBD_VehicleState.c:11`, `:80-83`). `$defs/entity` also has lock/fuel/ammo. An `entities[]`-only vehicle gets no state. T-675.2 authored roster rows emit both; the residual is hand-edited entities-only documents.

**Fix:** walk the entity twin as well, or document that roster is the only apply path and drop the entity-side schema claim.

---

## Attacked and FAILED to break

- **Gate vacuity / unexamined code (BLOCKER class):** `tbd_wave246_cold` exists; `missions_it` is 35 tables / 38 missions / 22 migrations. Wave gate at `d51a04518` base `f09955da0`: **GATE: PASS** (first run FAIL was rustfmt edition 2021 vs xtask 2024 only). `test api` cannot see this wave's files (no website-api diff) — expected, not the only step. Wave-level `cargo xtask mod compile`: OK, 5749 files / 11388 classes (wave 245 was 11363). Gate still does not run `mod compile` (T-946.17). Not re-filed.
- **T-678 enables AI / spawns groups:** failed. `TBD_GroupState.c` finds an existing `SCR_AIGroup` parent (T-677 arms waypointed LIVE groups) and does not ActivateAI. Groups with GRP attrs but no waypoints stay parked (T-677 / T-946.34). Residual of the shared AI gate, not a new global enable.
- **Export twins / T-946.26:** `TBD_GroupState.c`, `TBD_VehicleState.c`, `TBD_MissionParams.c` are byte-identical ASCII across framework and export (`cmp -s`).
- **UNREAD tripwires:** `combatMode` / `formation` / `fuel` / `lock` / `ammo` / `missionParams` have **no** `UNREAD_WIRE_FIELDS` rows (RETIRED comments). `size` expected 3 (T-681), `shape` expected 34 (T-673), `placementRadius`/`placementShape` expected 0 (T-679). Fire-once test retargets to `placementRadius`. `cargo test -p xtask unread_wire_field_tests` → 5 passed. `cargo xtask schema validate` → All contracts valid.
- **Schema still saying NOTHING reads this wave's fields:** `mission.schema.json` descriptions now `READ SINCE T-678/T-680/T-684 (2026-09-06)`. Entity lock/fuel/ammo wording notes roster is read, entities[]-only still unread.
- **Slices edited flatten.rs / schema_gates.rs / mission.schema.json:** merge commits do not. CC retired UNREAD and schema wording after land, as briefed.
- **T-684 lobby / Enforce `default` member:** no lobby. Wire key `default` is read via `ReadValue("default", …)` because Enforce cannot name a member `default`. Unknown symbols fail closed.

NIT (not a finding row): `authored_blocks_root` still cannot carry `tasks[]` (T-946.35) or `missionParams[]`. T-936.3 is the owned flatten slice.

## main_left_clean

- tracked tree clean at verify start (this file is the deliverable)
- `git rev-parse HEAD`: `d51a04518`
- `:3000` and `:8080` still LISTEN
