# T-941.8 — Plan

## Context
Vehicles spawn with engine-default fuel and no cargo. Queued T-675.2 covers the roster read and crew seats in the same
TBD_SpawnManager.c; this slice adds fuel and cargo and packs after T-941.2 and T-675.2.

## Approach
1. Verify on main: read the vehicle spawn path in TBD_SpawnManager.c; record as defect evidence.
2. After spawn: fuel full; cargo from a script-side default table keyed by vehicle class.
3. Roster fuel/inventory fields override the defaults when the wire carries them.
4. Missing cargo prefab → one warning naming it; the vehicle still spawns.
5. Perturbation: misspelled fuel API → compile red; restore, touch, green.

## Risks
- Default table drift from the faction library: keep it small and named per class.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.8`
- Checklist: roster vehicle spawns fuelled with class cargo; unknown cargo prefab warns once.
