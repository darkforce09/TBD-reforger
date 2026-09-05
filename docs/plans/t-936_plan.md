# T-936 — Plan (program)

## Context
Master audit S1 (2026-09-04): win/loss hardcoded (flatten.rs:2573-2582), radio synthesized (:1927-1960),
no tasks, weather timeline, audio, spawn modules or tactical graphics anywhere. Existing tickets cover
vehicles (T-675), triggers (T-676), waypoints (T-677), group AI (T-678), slot identity (T-674), markers
(T-673), objectives (T-212). The Play scenario dry run is deferred by operator 2026-09-04.

## Approach
1. T-936.1 win conditions + `mission/extensions.rs` (AUTHORED_BLOCKS passthrough: compile.rs → flatten).
2. T-936.2 tasks (after T-676), T-936.3 radio panel — both register in extensions.rs.
3. T-936.4 weather, T-936.5 audio, T-936.6 spawn modules, T-936.7 tactical graphics (after T-673).
   Each slice: schema block + golden, core model + validator, editor panel, Enfusion runtime script.

## Risks
- Every slice touches mission.schema.json → one slice per wave; the chain is the depends_on order.
- Mod runtime behaviour is unverifiable by CI → human checklist per slice; `cargo xtask mod compile` gates syntax.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask ci schema-validate`; `cargo xtask mod compile`; `cargo xtask platform wave gate --slice T-936.N`
