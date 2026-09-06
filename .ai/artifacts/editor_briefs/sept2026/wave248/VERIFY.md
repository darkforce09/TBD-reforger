# Wave 248 adversarial verify

HEAD at verify start: `fbae3f3ac` (UNREAD retirement). Base `21d384ccd`. Merges T-299 `f90297d7e`, T-679 `759b53354`, T-685 `db9af2402`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. `:3000` / `:8080` left listening. T-946.41 filing `7fb32c496` lands after this SHA.

**T-685 report's flatten-omits claim is half-wrong.** `flatten.rs:3168` clones `raw.rules` as `serde_json::Value`, so authored `zoneRules` keys including the six T-685 fields survive `/compiled` if the editor stored them. `rg placementRadius flatten.rs` is empty: `ModSlot` / `ModOrbatGroup` have no scatter fields. That hole is real and is T-946.41 (scatter only). T-946.36 still covers params / group AI / vehicle / entity state.

## Findings

### 1. MAJOR — flatten still drops placementRadius / placementShape (T-946.41)

`GetRawJson()` is the `/compiled` body. `ModSlot` (`flatten.rs:142`) and `ModOrbatGroup` (`:292`) have no `placementRadius` / `placementShape`. Live compile never reaches `TBD_PlacementScatter.c`. Hand-staged `schema-1_3-wire-fields.json` does. Group scatter is a shared offset because groups have no authored x/z (NIT, not re-filed).

### 2. MAJOR — flatten still drops params / group AI / vehicle / entity state (T-946.36)

Unchanged this wave. Not re-filed. Zone volume keys are **not** this class (Value clone).

### 3. MAJOR — tasks and radio panels never mount (T-946.33 + T-946.39)

Unchanged. Not re-filed.

### 4. NIT — destroy diagnose stays XZ on authored entities[]

`TBD_ObjectiveRegistry.c:753` live query uses `ContainsOrigin` on `GetOrigin()` (has Y). Authored `entities[]` rows still have no Y, so the diagnose path cannot apply AGL. Not filed.

---

## Attacked and FAILED to break

- **T-299 phantom pad:** `factions.minItems` is 1 (`mission.schema.json` ~line 70). Remaining `minItems: 2` are `playerRange` (`:184`) and `$defs/polygon` pair (`:577`), not factions. `TBD_MissionValidator.c` has no "at least two" faction warning. Two-faction Class-R path untouched.
- **T-685 apply is not a dead API:** both `TBD_ObjectivesComponent.c` trees call `ContainsAgl` / `ResolveActingFaction` / `EnemyContestsHold`. Owns widened `189275663`. `TBD_ZoneVolume.c` twins byte-identical (`cmp -s`).
- **T-679 twins:** `TBD_PlacementScatter.c` byte-identical. `SpawnManager` calls `ForSlot` before height.
- **UNREAD tripwires:** six T-685 rows and two T-679 rows retired (baselines were 0). `shape` re-pinned **34 → 36** (`TBD_PlacementScatter.Scatter` local `shape` parameter, two identifiers; `why` contains `different` / `unrelated`). `size` stays 14. Fire-once retargeted to `vehicleClasses` / T-689. `cargo test -p xtask unread_wire_field_tests` → 5 passed. `cargo xtask schema validate` → All contracts valid (13 unread at baseline).
- **Slices edited schema_gates.rs / mission.schema.json:** merge commits do not. CC retired UNREAD after land.
- **Export twins / T-946.26:** new `.c` files are byte-identical ASCII. Loader twins still differ emdash vs hyphen in comments (pre-existing).
- **T-946.35 flatten drops tasks[]:** still cancelled (T-936.3). Do not re-file.

## main_left_clean

- tracked dirty at verify start: none required for the UNREAD commit
- `:3000` and `:8080` still LISTEN
