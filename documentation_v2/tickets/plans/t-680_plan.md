# T-680 — Plan

## Context

ATTR-FIELD-OBJ-LOCK/FUEL/AMMO are on the wire since T-706 with no Enfusion reader (word-boundary grep: no vehicle-lock semantics in `apps/mod`). Vehicle spawn itself lands in T-675.2 (order 4321); this slice packs after it.

## Approach

1. Verify on main: no lock/fuel/ammo application in `TBD_SpawnManager.c`.
2. New `Vehicles/TBD_VehicleState.c`: `Apply(vehicle, lock, fuel, ammo)` using the vehicle controller, fuel manager and ammo components; call it from the vehicle spawn site in `TBD_SpawnManager.c`.
3. Unset attributes leave engine defaults; compile.

## Risks

- Ammo application differs between turret and cargo magazines; document the chosen scope in the report.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-680`
