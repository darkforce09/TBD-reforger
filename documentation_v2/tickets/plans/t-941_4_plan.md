# T-941.4 — Plan

## Context
TBD_ObjectivesComponent.c:813 pumps per-player chat (SendPrivateMessage); no HUD element. After T-941.3.

## Approach
1. Verify on main: read :813 and the call sites; record as defect evidence.
2. UI/Hud/TBD_ObjectiveHud.c + UI/layouts/TBD_ObjectiveHud.layout: objective list with state icons, capture bar.
3. TBD_ObjectivesComponent.c: replicate state + capture progress; drop the pump; keep the objective-complete chat line.
4. Layout path registered through the HUD script's own constant (TBD_UILayouts.c belongs to .3).
5. Perturbation: bind the bar to a missing widget → compile red; restore, touch, green.

## Risks
- Replication rate: progress at a bounded tick, not every frame.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.4`
- Checklist: bar fills in a capture zone; icon updates on completion; no per-tick chat.
