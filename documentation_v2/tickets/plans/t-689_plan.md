# T-689 — Plan

## Context

FNF v4's play-zone AIR flag (false) lets aircraft leave the play area deliberately; TBD's `zoneRules` has graceSeconds/warnEverySeconds/penalty but no vehicle-class axis. Pairs with T-685 (same `zoneRules` reader), which packs first.

## Approach

1. Verify on main: `rg -n 'vehicleClass|aircraft' apps/mod/tbd-framework/Scripts/Game/TBD/Zones/` is empty.
2. `TBD_MissionLoader.c`: bind the per-class penalty axis on boundary/base_protection zones; new `Zones/TBD_PlayAreaVehicleAxis.c`: classify the occupant (on-foot, ground, air) and return the effective penalty; branch in `TBD_ZoneRegistry.c`.
3. Compile; defaults reproduce today's behaviour exactly.

## Risks

- Vehicle classification by prefab tag may miss modded vehicles; default to ground.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-689`
