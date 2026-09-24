# T-291 — Plan

## Context
Five contract fields have no implementation; flatten.rs:2104 even trims respawn/spectatorPolicy. The Spectator
subsystem shipped without a policy input. Implement the three runtime fields, annotate the two editor-only ones.

## Approach
1. `crates/map-engine-core/src/mission/flatten.rs`: emit settings.spectatorPolicy, settings.nightVision,
   environment.windDirDeg (stop trimming); golden with all three; test red on main first.
2. `Gamemode/TBD_FrameworkManager.c`: parse the three; apply wind direction in weather setup; expose policy.
   `Spectator/TBD_SpectatorController.c`: enforce policy (none / own-side / all). `cargo xtask mod compile`.
3. Ledger rows (T-290's EMIT_LEDGER if landed, else a doc-comment table) for factions.color, roles.radio, layers.
4. Perturbation: re-trim spectatorPolicy → emit test red; restore, `touch`, green.

## Risks
- settings.respawn semantics overlap the T-181 respawn work; emit only, reader stays with that lineage — report it.

## Verification
- `cargo test -p map-engine-core --all-features mission::flatten` · `cargo xtask mod compile`
- `cargo xtask platform wave gate --slice T-291` · human checklist: spectator policy respected in game.
