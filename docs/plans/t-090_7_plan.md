# T-090.7 — Eden AI world object schema (exact field contract)

## Context

Mission Creator will expose AI inside the Eden-style editor; the AI must read the
world base layer (1M+ map objects) with the certainty a human gets from selecting
an entity in Workbench — what/where/size/tactical/trust. The schema slice pinned
`ResolvedWorldObject`; this runtime slice makes it reach the frontend AI code.

## Approach

Wire the resolver in `apps/website/frontend/src/editor/world_assets` (new `resolved.rs`, registered in `mod.rs`; prefab data from `world_host.rs`) to produce
`ResolvedWorldObject` exactly as
`packages/tbd-schema/schema/map-object-resolved.schema.json` defines it — prefab +
instance join, required typed fields (type, label, position, bounds, tactical
flags, audit trust) — and expose it as the AI tool shape. No parallel field names
invented in frontend AI code; the schema is the single contract.

## Risks

Field drift between the schema JSON and hand-written frontend types is the whole
failure class — codegen/golden tests must pin the shape; audit-trust fields depend
on T-090.4/.6 outputs existing, so absent audits must render as explicitly
unknown, never as trusted placement.

## Verification

`ResolvedWorldObject` instances validate against the committed schema; frontend
AI reads compile against the generated/pinned shape only; a sample object
round-trips prefab + instance → resolved with every required field populated
(spec: `docs/specs/Mission_Creator_Architecture/t090_eden_ai_world_object_schema.md`).
