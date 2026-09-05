# T-212 — Plan

## Context

`objectivesById` is a dead container: it compiles into the editor superset (`compile.rs:228`) but is not a schema root property and `flatten.rs` never mentions it. What works is objectives-as-zones (`TBD_ObjectiveRegistry.c:36-42` keys on `zone.type`). The corpus converges on "an objective is a typed, placed, per-side entity with a uniform attribute spine" (WOG `WMT_Task_Point`, FNF v4). T-685 (volumes) packs first and shares `TBD_ObjectiveRegistry.c`. The T-257 edge (undo scope, website) was dropped — this slice is the mod reader.

## Approach

1. Decide the shape in the report: WOG's parameter spine on top of the zone-keyed registry, per-side framing, stable `uid`, scripting hatch behind advanced disclosure.
2. `TBD_ObjectiveRegistry.c`: resolve typed per-side objectives from the compiled payload (keys T-706 shipped); keep the zone-typed path working.
3. Mark inferred WOG semantics as inferred in comments; the SPA authoring UI is a follow-on slice with its own owns.

## Risks

- Designing against the dead `objectivesById` instead of the wire; the registry is the only consumer.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-212`
