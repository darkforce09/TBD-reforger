# T-679 — Plan

## Context

Three ids (OBJ placement radius/shape, GRP placement radius) are on the wire since T-706; spawn ignores them. Cheapest coherent reader in the workbench program.

## Approach

1. Verify on main: `rg -n 'placementRadius|placement' apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` shows no use.
2. New `Backend/TBD_PlacementScatter.c`: deterministic `Scatter(center, radius, shape, seed)`; call it from the slot and group spawn sites in `TBD_SpawnManager.c`; zero radius returns the center.
3. Compile; scatter distribution unit-checked by a script-side self-test if the compile harness runs one, else described in the report.

## Risks

- Scattered positions may land inside geometry; clamp to navmesh/terrain height where the API allows.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-679`
