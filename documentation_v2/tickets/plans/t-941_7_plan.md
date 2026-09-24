# T-941.7 — Plan

## Context
TBD_RadioTuner.c:49-51 NO_BACKBONE, :142-143 set when GetBackbone() is null; TBD_RadioComponent.c:135 boot warning;
TBD_Dev_POC.ent lacks RadioManagerEntity. Operator deferral, quoted: RadioManagerEntity world edit — deferred by
operator 2026-09-04. This slice ships the script fallback and the checklist only.

## Approach
1. Verify on main: read :142-143 and :135; record as defect evidence.
2. TBD_RadioTuner.c: NO_BACKBONE → script-side channel table (mission radioPlan when present, else defaults); tune works.
3. TBD_RadioComponent.c:135: warning names the world, RadioManagerEntity and the fallback, once per boot.
4. Perturbation: wrong return type from the fallback lookup → compile red; restore, touch, green.

## Risks

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot` (TBD_Dev_POC)
- `cargo xtask platform wave gate --slice T-941.7`
- Checklist: two players tune the same fallback channel and hear each other; one warning line.
- Operator checklist (deferred world edit): open TBD_Dev_POC.ent in Workbench, add RadioManagerEntity, save, re-run boot.
