# T-940 — Plan

## Context
Master audit S5 (2026-09-04) verified against main: waitlist seats, tombstones, schedule cascades, flat telemetry,
pool config, audit stream, users pager, vehicle mutations, wiki features, mortar model, rcon actions, tiers, telemetry
events. Thirteen slices under `apps/website/api/src/`; migrations from 0022.

## Approach
1. Wave A: T-940.1 waitlist, .4 telemetry fold, .5 pool config, .6 audit notify, .7 users pager, .10 ballistics.
2. Wave B: .2 tombstones (after .1), .13 events (after .4), .11 rcon (after .7).
3. Wave C: .3 cascades (after .2), .8 vehicles (after .2).
4. Wave D: .9 wiki (after .8), .12 tiers (after .3).

## Risks
- events.rs chain (.1 → .2 → .3 → .12) serializes four slices; keep each diff local to its handler.
- Migration numbers collide with sibling programs: take the next free number at ship time.

## Verification
- `cargo xtask db test-it`; `cargo xtask platform wave gate --slice T-940.N` per child
- `cargo xtask mk ci-local-leptos` for slices touching the frontend
