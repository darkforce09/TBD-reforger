# T-941.2 — Plan

## Context
TBD_LobbyScreen.c:211-212 acknowledges the deploy race; :492-507 closes the screen instead of locking. TBD_SpawnManager.c
m_bAutoDeploy fires ~250 ms into LOBBY. Priority 0.

## Approach
1. Verify on main: read :211-212 and the m_bAutoDeploy timing; record as defect evidence.
2. TBD_SpawnManager.c: deploy on LOBBY→BRIEFING, once per player id (deployed set).
3. TBD_LobbyScreen.c: DEPLOY disabled with a reason while pending; reopen from the pause menu to change slot (despawn + redeploy).
4. Dated decision replaces the :211-212 note.
5. Perturbation: mismatched type on the deployed set → compile red; restore, touch, green.

## Risks
- TBD_SpawnManager.c shared with T-941.8 and queued T-675.2: separate waves.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.2`
- Checklist: one body per holder on BRIEFING; double-click never doubles; reopen + change slot redeploys.
