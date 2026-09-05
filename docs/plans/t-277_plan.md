# T-277 — Plan

## Context
444 of 1,623 everon prefabs are `fallback` against 73 rules; vegetation/utility census is zero and roads are not
counted although roads.json.gz ships 888 segments. Rules are first-match, so appending cannot break existing lanes.

## Approach
1. Extract the fallback resource names from `packages/map-assets/everon/objects/type-inventory.json`; group by
   path prefix (Vegetation/, Props/Utility/, Structures/...).
2. Append one rule per group to `packages/tbd-schema/rules/prefab-classify.json` with gameplay, spatial and
   render.iconKey per t090_eden_ai_world_object_schema.md; keep `_turret`/CannonWreck rules ahead of vehicle rules.
3. Local `tbd-tools world build-objects` over the staged export to measure; paste the before/after counts; do not
   commit the rebuilt catalogue (T-935.13 owns the cutover build).
4. Perturbation: delete one appended rule → coverage test red; restore, `touch`, green.
## Risks
- Road census zero may be a counter bug in build.rs:746 rather than a rule gap; if so, report found_not_fixed
  (build.rs is not owned here).
## Verification
- `cargo xtask ci schema-validate` · `cargo xtask ci verify type-inventory` · `cargo xtask ci verify map-object-enums`
- `cargo xtask platform wave gate --slice T-277`
