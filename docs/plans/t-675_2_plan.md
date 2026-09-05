# T-675.2 — Plan

## Context

T-675.1 puts `vehicles[]` on the wire; the mod loader has no field for it and `TBD_SpawnManager.c` spawns bodies only, so authored crews never sit in vehicles. T-674.2 lands the identity binding and the validator 1.3 entry first on the same files.

## Approach

1. Verify on main: `rg -n 'vehicles' apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` shows no binding.
2. New `TBD_MissionVehicleStruct.c` (Backend/) for roster rows and seats; bind `vehicles[]` in `TBD_MissionLoader.c`.
3. `TBD_SpawnManager.c`: spawn each vehicle after bodies, seat the referenced slots; unknown prefab or unresolved ref logs and skips that row.

## Risks

- Seat-role names differ per vehicle prefab; fallback is driver-first fill with a warning.
- Seating cannot be seen headlessly — human checklist.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-675.2`
