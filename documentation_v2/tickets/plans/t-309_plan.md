# T-309 — Plan

## Context
T-217 made multi-squad Apply refuse loudly; the schema (additionalProperties:false, flat roles/vehicles) and the
flat FactionDoc (core/dto.rs:745) are why. Both editor ops (entity.rs:2353 apply, :2913 flatten) need a squad level.

## Approach
1. `packages/tbd-schema/schema/faction-library.schema.json`: optional `squads[]` of {callsign, roles[], vehicles[]};
   keep flat fields required for backward compatibility; add a golden with two squads; `schema-validate`.
2. `apps/website/frontend/src/core/dto.rs`: `FactionDoc.squads: Option<Vec<SquadDoc>>`.
3. `state/operations/entity.rs`: faction_doc_from_side_core emits squads; orbat_apply_faction recreates each squad
   (existing orbat_add_squad/orbat_add_slot mutators); remove the T-217 refusal once the round-trip test is green.
4. Perturbation: skip the squads emit → round-trip test red; restore, `touch`, green.
## Risks
- Templates saved by old clients lack squads → treat as one squad (test it).
- entity.rs is shared with T-141; rebase if it landed first.

## Verification
- `cargo xtask ci schema-validate` · `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-309`
