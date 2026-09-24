# T-936.6 — Plan

## Context
No dynamic spawn or garrison module exists (audit S1); AI is placed statically. Related: T-299 phantom
opfor, T-678 group AI, T-676 triggers (used as spawn triggers).

## Approach
1. Schema `spawnModules[]` {id, kind wave|garrison, factionKey, groupTemplate, x/z or zoneId, count, ...}.
2. `mission/spawn_modules.rs` (new, in mission/mod.rs): model + validator; register in extensions.rs.
3. `panels/spawn_modules.rs` (new, in panels/mod.rs): module editor, undoable.
4. `Gamemode/TBD_DynamicSpawner.c` (new): waves on interval/trigger up to maxAlive; garrison spawns once.
5. Perturbation: allow x/z and zoneId together → exclusivity test red; restore, touch, green.

## Risks
- Server load from waves — maxAlive default and a hard cap in the validator.
- Group templates must exist in the faction catalog; validate by key.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask ci schema-validate`; `cargo xtask mk ci-local-leptos`
- `cargo xtask mk leptos-gates`; `cargo xtask mod compile`; `cargo xtask platform wave gate --slice T-936.6`
