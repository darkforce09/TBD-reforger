# T-941 — Plan

## Context
Master audit S6 (2026-09-04) verified against main: safestart arms only SAFE_START, lobby deploy race, no END/DEBRIEF
screens, chat-only objectives, spectator range 0 = unlimited, link code in public chat, radio NO_BACKBONE, naked
vehicles. Eight script slices under `apps/mod/tbd-framework/`. Operator: agents may edit the Enfusion mod scripts;
gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.

## Approach
1. Wave A (disjoint owns): T-941.1 safestart, .2 lobby deploy, .3 END/DEBRIEF, .5 spectator, .6 link code, .7 radio.
2. Wave B: T-941.4 objective HUD (after .3), T-941.8 vehicles (after .2 and queued T-675.2; TBD_SpawnManager.c).
3. Each slice: defect evidence from the anchor lines, compile, world-boot, deliberate compile break as perturbation.

## Risks
- No script unit-test lane: the compile gate is the only automated proof; checklists must be run by a human.
- TBD_SpawnManager.c three-way share (.2, .8, T-675.2): strictly serialized by the packer.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.N` per child
