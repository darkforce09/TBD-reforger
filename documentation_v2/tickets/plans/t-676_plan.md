# T-676 — Plan

## Context

T-079 shipped the trigger geometry palette and T-706 the schema keys; nothing in the mod activates triggers. Twelve ATTR-FIELD-TRG ids and the sixteen `zoneRules` keys need a runtime with an activation model (condition, repeat, timeout, owner side) and an effects model.

## Approach

1. Read how `TBD_MissionLoader.c` / `TBD_ZoneRegistry.c` expose `zoneRules` today and map each trigger key to a semantic before writing code.
2. New `Zones/TBD_TriggerRuntime.c`: a server-side component subscribed to zone-registry presence events; evaluates condition/repeat/timeout/owner and fires the effect keys.
3. If a trigger key has no loader field, report it under `files_outside_owns` with the exact field — do not widen the schema.

## Risks

- Replication: effects that touch players must run server-authoritative; keep the runtime server-only.
- In-game activation is a human-checklist item; the gate is the headless compile.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-676`
