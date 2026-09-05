# T-299 — Plan

## Context
flatten.rs:2470-2472 pads a stub opfor because the schema demands two factions; the mod validator then warns that
nobody can play it. The document is valid and wrong. Decision: permit one faction end to end.

## Approach
1. Fixture with one faction → compile on main → paste the padded opfor (defect verified).
2. `packages/tbd-schema/schema/mission.schema.json`: factions minItems 1; golden; `schema-validate`; `schema-codegen`.
3. `crates/map-engine-core/src/mission/flatten.rs`: delete the pad block; single-faction test; two-faction goldens
   byte-identical (existing tests).
4. `Backend/TBD_MissionValidator.c`: accept one faction (drop the ≥2 rule if present); `cargo xtask mod compile`.
5. Perturbation: restore the pad → single-faction test red; restore, `touch`, green.

## Risks
- Mod code that indexes factions[1] (briefing/ORBAT) — grep apps/mod for hard-coded second faction; report any
  found_not_fixed outside the owned validator.

## Verification
- flatten tests · `schema-validate` · `schema-codegen` · `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-299`
