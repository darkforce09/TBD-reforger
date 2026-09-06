# REPORT T-681 — Entity states: health, allow-damage, show-model, size, stamina

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-681
slice/T-681
```

First action of the run, before any edit. Code commit `7a73689fdfed5f2aa29d9fc1bb9c46dcf424fef2`. Report commit follows this file.

## Stamina API answer (before coding)

**Reforger does not expose a per-character stamina enable/disable toggle.** Schema `stamina` is a **boolean** ("whether stamina is enabled"), not a 0..1 fraction.

| claim | evidence |
|---|---|
| Component | `CharacterStaminaComponent` extends `BaseStaminaComponent`. Game wrapper `SCR_CharacterStaminaComponent` is an empty subclass. |
| Getter | `BaseStaminaComponent.GetStamina()` — `apps/mod/vanilla_reference/Scripts/Game/generated/Base/BaseStaminaComponent.c:14` (`proto external float GetStamina();`). No setter in that generated class. |
| Drain restore, not a toggle | CRF override calls `AddStamina(staminaToRestore)` inside `OnStaminaDrain` (`apps/mod/crf_framework/Scripts/Game/Systems/VanillaOverrides/Character/CRF_SCR_CharacterStaminaComponent.c:20`). That compensates drain during safestart; it is not EnableStamina. |
| Enable toggle | `rg EnableStamina\|SetStamina\|DisableStamina\|SetUnlimitedStamina` over `apps/mod/vanilla_reference` `.c` files: **zero** enable/disable APIs. `StaminaSystem` generated class is empty. |

Therefore stamina is **not applied**. Authored `stamina: true` logs once per apply pass (`LogStaminaSkip`) and is recorded in found_not_fixed. No no-op setter was invented.

Brief listed stamina with the numeric `ABSENT = -1000000` sentinel. Live schema (`packages/tbd-schema/schema/mission.schema.json:796`) types it as boolean. Followed the schema (deviation below).

## defect_verified_on_main

Worktree branched from main at `dbb8d4998` (T-942 twin widen). Proved before writing:

| claim | path:line | command |
|---|---|---|
| `TBD_EntityState.c` does not exist | (file absent both trees) | `test ! -f apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_EntityState.c` → missing; same for tbd-export |
| No `allowDamage` / `showModel` / `TBD_EntityState` / `SetHealthScaled` reader in framework TBD scripts | 0 files | python walk of `apps/mod/tbd-framework/Scripts/**/*.c` |
| `health` hit is English, not OBJ-HEALTH | `TBD_SpawnManager.c` comment "perfectly healthy dedicated server" | python context around first `health` |
| `TBD_MissionEntityStruct` has no health/allowDamage/showModel/size/stamina | `TBD_MissionLoader.c:269-286` | fields are alias/uid/x/z/headingDeg/faction only |
| Spawn path applies vehicle state, not entity state | `TBD_SpawnManager.c:1031` | `TBD_VehicleState.ApplySpawned();` then `if (built <= 0) return;` — no EntityState |

Not already fixed. Implemented.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_EntityState.c` | NEW (350 lines). Wire structs 38–95. Twin index 100–138. `ApplySpawned` 144. `Apply` 207. Second parse `GetRawJson` 230. `SetHealthScaled` 303. `EnableDamageHandling(true)` 316. `SetFlags(EntityFlags.VISIBLE)` 322. `SetScale` 334. Stamina skip-log 343. | T-680-style second `JsonLoadContext` pass over `entities[]` health/allowDamage/showModel/size/stamina. Apply at spawn. Unset numerics are `ABSENT = -1000000`. Bools apply only when bound true. Stamina logged, not applied. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_EntityState.c` | NEW, byte-identical ASCII twin | T-946.26 mandatory twin. `mirror_lockstep` walks export only. `non_ascii=0`. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 663 `ResetIndex`; 724 `RecordSpawn` | Record each spawned `entities[]` body so Apply can join on uid / alias\|x\|z without an AABB guess. Reset on the same reload boundary as the vehicle roster. Struct **not** grown. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 663, 724 | Same call sites, ASCII comments. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1032–1034 | After `TBD_VehicleState.ApplySpawned` (and still before `built <= 0` return) `TBD_EntityState.ApplySpawned();` so an entity-only mission still applies. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1032–1034 | Same call site, ASCII comments. |

No `packages/tbd-schema` edits. No `flatten.rs`. No `schema_gates.rs`. No `TBD_VehicleState.c`. No `TBD_TaskStateMachine.c`. No editor UI.

### Apply mapping

- **health** (number 0..1): `DamageManagerComponent.SetHealthScaled`. 0 is authored destroyed. Missing component → log and skip. Applied last so size/visibility still land on a 0-health body.
- **allowDamage** (bool): `EnableDamageHandling(true)` only when bound true. Authored false and omit are indistinguishable (T-676 / T-946.37).
- **showModel** (bool): `IEntity.SetFlags(EntityFlags.VISIBLE, false)` only when bound true. Same presence hole. Hide (the interesting Eden case) cannot be distinguished from omit.
- **size** (number, exclusiveMinimum 0): `IEntity.SetScale`. `<= 0` skipped. If `GetScale` does not match, log (prefab may not support size) rather than failing the spawn.
- **stamina** (schema bool): not applied; see stamina answer.

## perturbation

Broke `SetHealthScaled` → `SetHealthScaled_T681` in the **framework** copy only. `cargo xtask mod compile` RED, verbatim:

```
FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Backend/TBD_EntityState.c:303: Undefined function 'DamageManagerComponent.SetHealthScaled_T681'
------------------------------------------------------------
1 error(s) in TBD sources, 19 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:324: Error in parameters
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:333: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:335: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Syntax error
  Scripts/Game/Map/ComponentsUI/SCR_MapUIElementContainer.c:443: Incompatible parameter 'on delete'
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:26: Error in parameters
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:29: Can't find variable 'friendlyDefaultMapping'
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:42: Can't find variable 'friendlyDefaultMapping'
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:52: Can't find variable 'friendlyDefaultMapping'
  … 9 more
red_exit=1
```

Restored the identifier, `touch`ed `TBD_EntityState.c`, twins identical again (`cmp` equal).

**restored_green:**

```
OK: compiled clean
    Module: Game; loaded 5750x files; 11396x classes
    Compiling Game scripts took: 887.308000 ms
    0 warning(s) in TBD sources
green_exit=0
```

(First green, before perturbation, was the same verdict at 857.310000 ms, 5750 files / 11396 classes.)

## gate_verdict_tail

`cargo xtask platform wave gate --slice T-681` — last 15 lines:

```
  T-439 objects aliases    PASS
  T-444 wiki seed          PASS
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict FAIL @ 7a73689fdfed recorded: .ai/artifacts/verdicts/T-681.json
SLICE GATE: FAIL
```

Did **not** end `SLICE GATE: PASS`. Sole red step: `schema` (height-labels SKIP is the worktree LFS pointer — environmental, per brief). Real failure is unread-wire-fields, as the command-center note predicted. Did **not** edit `xtask/src/schema_gates.rs`. Did **not** re-pin.

`cargo check` / wasm32 / fmt / clippy: PASS. Cheap gate did not compile Enfusion (T-946.17); that is `mod compile` below.

Unread-wire-fields after the reader (`cargo xtask schema validate`):

```
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'allowDamage' now has 4 mod identifier(s) (baseline 0)
        'showModel' now has 4 mod identifier(s) (baseline 0)
        'stamina' now has 3 mod identifier(s) (baseline 0)
        'health' now has 9 mod identifier(s) (baseline 0)
        'size' now has 14 mod identifier(s) (baseline 3)
```

CC retires allowDamage/showModel/stamina/health after merge; re-pins `size` (T-673 marker.size baseline 3 plus this entity-size reader).

## mod_compile_verdict

After restore (the shippable tree):

```
OK: compiled clean
    Module: Game; loaded 5750x files; 11396x classes
    Compiling Game scripts took: 887.308000 ms
    0 warning(s) in TBD sources
```

## files_outside_owns

[]

EnfusionMCP scripts were already present untracked (T-946.27); not committed.

## found_not_fixed

| path:line | repro |
|---|---|
| `TBD_EntityState.c:48,47` `allowDamage` / `showModel` | Bool presence hole (T-676 / T-946.37). Omit and authored-false both bind `false`. Apply only the true case. Authored `allowDamage: false` (invincible) and `showModel: false` (hidden) **cannot** be distinguished from omit, so they leave engine defaults. |
| `TBD_EntityState.c:50,343` `stamina` | No Reforger per-character stamina enable toggle (see stamina answer). Authored `stamina: true` logs once per apply pass and is skipped. Authored `stamina: false` is indistinguishable from omit. |
| `flatten.rs` (not owned) | `/compiled` body does not emit entity health/size/etc. Hand-staged 1.3 JSON / golden still reach the reader (same as T-678/T-680). Not fixed, per brief. |
| `xtask/src/schema_gates.rs` unread rows | Reader landed; gate still pins expected 0 (and size 3). CC retires/re-pins after merge. Not edited. |

## deviations

1. Brief grouped `stamina` with numeric `ABSENT = -1000000`. Live `mission.schema.json:796` types `stamina` as **boolean**. Implemented as bool; sentinel used only for `health` and `size`.
2. Generic rule asked for `SLICE GATE: PASS`. Command-center note said unread/size pins **will FAIL** and forbade re-pin. Gate failed only on those pins. Did not re-pin.

## commits

- `7a73689fdfed5f2aa29d9fc1bb9c46dcf424fef2` — T-681: apply authored entity health, damage, model, and size at spawn.

## manual_checklist

1. Place an `entities[]` row with `health: 0.5` in hand-staged 1.3 JSON (flatten will not emit it); confirm the spawned body is at half health after boot.
2. Same row with `size: 2`; confirm the world object is double native scale.
3. Authored `allowDamage: true` on a damageable prefab; confirm damage handling is on (usually already the default).
4. Authored `showModel: true`; confirm the mesh is visible (usually already the default).
5. Confirm an entity that **omits** all five keys is unchanged vs pre-T-681 (engine defaults).
6. Authored `stamina: true` on a character-shaped entity: boot log contains one `[TBD][EntityState] authored stamina=true but Reforger exposes no per-character stamina enable toggle` line and stamina behaviour is unchanged.
7. Authored `allowDamage: false` / `showModel: false`: expect **no** invincibility / hide (presence hole) — visual confirmation that the hole is real.

## twins_confirmed

| path | exists |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_EntityState.c` | yes (NEW, 12340 bytes, ASCII) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_EntityState.c` | yes (NEW, 12340 bytes, ASCII, `cmp` identical to framework) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (edited) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (edited) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (edited) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (edited) |
