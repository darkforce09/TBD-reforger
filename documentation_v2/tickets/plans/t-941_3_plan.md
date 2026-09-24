# T-941.3 — Plan

## Context
TBD_UILayouts.c:28,31 registers only SCREEN_SHELL and LIST_ROW; stage hooks exist in TBD_FrameworkManager.c:1022-1295
with no screen. Layouts dir: apps/mod/tbd-framework/UI/layouts/. Priority 0.

## Approach
1. Verify on main: read :28-31 and the four hooks; record as defect evidence.
2. Layouts: TBD_EndScreen.layout, TBD_DebriefScreen.layout reusing the shell + list-row widgets.
3. Scripts: UI/End/TBD_EndScreen.c (winner + reason), UI/End/TBD_DebriefScreen.c (scoreboard from TBD_ResultsReporter counters).
4. TBD_UILayouts.c constants; TBD_FrameworkManager.c opens on END/DEBRIEF, closes on the next stage.
5. Perturbation: missing widget name in TBD_EndScreen.c → compile red; restore, touch, green.

## Risks
- Screens must never block a stage change; close on transition unconditionally.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.3`
- Checklist: banner on END; scoreboard on DEBRIEF with kills/deaths; both close on the next stage.
