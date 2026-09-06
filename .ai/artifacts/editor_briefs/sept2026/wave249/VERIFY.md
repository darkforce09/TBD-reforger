# Wave 249 adversarial verify

HEAD at verify start: `6a01b4f9c` (UNREAD retirement). Base `dbd4a3e5c`. Merges T-941.1 `c3866cddb`, T-310 `37d718e60`, T-689 `bea43cb93`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. `:3000` / `:8080` left listening.

## Findings

### 1. MAJOR — empty `vehicleClasses` list confines everyone (T-946.42)

Schema defines authored `[]` as the inverse of absent (confine nobody). `JsonLoadContext` + `ref array<string>` binds absent and `[]` as `Count()==0`. T-689 treats that as confine-all (today). Hand-staged `[]` cannot invert the axis.

### 2. MAJOR — flatten still drops params / group AI / vehicle / entity state / scatter (T-946.36, T-946.41)

Unchanged. Zone `rules` still clone as Value, so `vehicleClasses` authored on a zone does reach `/compiled`. Not re-filed.

### 3. MAJOR — tasks and radio panels never mount (T-946.33 + T-946.39)

Unchanged. Not re-filed.

### 4. NIT — FrameworkManager comment still says SAFE_START-only

`TBD_FrameworkManager.c:787-789` still comments that SAFE_START arms and anything else lifts. Call is every-transition `OnStageChanged`; behaviour is T-941.1. Not filed.

### 5. NIT — non-primary attachment edges still drop

Same class as optic/magazine: flatten reads `(0,primary)` only. Schema description states it. Not re-filed.

---

## Attacked and FAILED to break

- **T-941.1 arm set:** `OnStageChanged` arms LOBBY/BRIEFING/SAFE_START and `Lift`s otherwise. Countdown starts only on SAFE_START so untouched `TickCountdown` stage-drift cannot drop the lobby shield. Twins match after ASCII fold. `mod compile` perturbation `stage == "LOBBY"` went red (`Incompatible parameter 'stage'`).
- **T-310 emit:** `mod_slot_loadout` at `flatten.rs:2012` / emit `:2056`. Golden `arsenal_suppressor_edge_reaches_compiled_gear`; empty `attachments: []` omits the compiled key. Schema insert is surgical (`attachments` on gear). Slot struct bind + TestNPC copy required; owns widened.
- **T-689 apply:** new `TBD_PlayAreaVehicleAxis.c` twins byte-identical. Loader bind + ZoneRegistry apply. Omit `aircraft` → aircraft may leave; infantry after dismount is confined. `cmp -s` on the new file.
- **UNREAD:** `vehicleClasses` retired (baseline 0). Fire-once retargeted to `framing` / T-212. `area` stayed 13. `cargo test -p xtask unread_wire_field_tests` → 5 passed. `cargo xtask schema validate` → All contracts valid (12 unread at baseline).
- **Wave-level `mod compile`:** OK, 5753 files / **11419** classes, 0 TBD warnings.
- **Slices edited schema_gates.rs:** T-689/T-941.1 merge commits do not. T-310 owns schema and edited `mission.schema.json` only (allowed). CC retired UNREAD after land.

## main_left_clean

- `:3000` and `:8080` still LISTEN
