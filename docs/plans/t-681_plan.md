# T-681 — Plan

## Context

Five OBJ attrs (health, allow-damage, show-model, size, stamina) have no runtime destination: `mission.schema.json:72` records zero readers for `entities[]`. Stamina depends on a per-character Reforger API that must be answered first.

## Approach

1. Answer the stamina API question in the report (component and setter, or "not exposed" with evidence) before coding.
2. `TBD_MissionLoader.c`: bind `entities[]` into a struct; new `Backend/TBD_EntityState.c`: apply the five states; hook it from the entity spawn site in `TBD_SpawnManager.c`.
3. Compile; unset attrs leave defaults.

## Risks

- `show-model` and `size` may be unsupported on some prefabs; log and skip rather than fail the spawn.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-681`
