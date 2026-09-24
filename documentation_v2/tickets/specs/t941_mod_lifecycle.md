# T-941 — Enfusion mod lifecycle: safestart, lobby, screens, HUD, spectator, link, radio, vehicles

Owner: command center. Source: master audit S6 (2026-09-04), verified against main @ 072988d57 (README.md in this
directory). Scope: TBD-Reforger only. Executor: claude-code. Mod dir `apps/mod/tbd-framework/Scripts/Game/TBD/`;
layouts in `apps/mod/tbd-framework/UI/layouts/`. No script unit-test lane exists: the gate proves compile + boot;
in-game behaviour is the MANUAL checklist of each block.

Operator authorization (quoted): agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game
behaviour goes on a human checklist. Operator deferral (quoted): RadioManagerEntity world edit — deferred by operator
2026-09-04 — T-941.7 ships the script fallback and a checklist only.

## 1. Related existing tickets — referenced, never re-minted
T-675.2 vehicle reader (crew seats) · T-311 leaderboard tie-breaker · T-136 AAR replay · T-393 (shipped) identity link ·
T-070 vehicles placeable · T-302 equip verify. False claim, no ticket (false_claims.md): TickCountdown loops at 0.

## 2. Verified anchors (2026-09-04)
| Finding | Anchor | Verdict | Slice |
|---|---|---|---|
| safestart arms only SAFE_START | Gamemode/TBD_SafestartManager.c:252-265 (:248-250) | TRUE | T-941.1 |
| lobby deploy race, no lockout | UI/Lobby/TBD_LobbyScreen.c:211-212, :492-507; TBD_SpawnManager m_bAutoDeploy | PARTIAL | T-941.2 |
| END/DEBRIEF no layouts | UI/TBD_UILayouts.c:28,31; Gamemode/TBD_FrameworkManager.c:1022,1123,1162,1295 | TRUE | T-941.3 |
| chat-only objectives | Objectives/TBD_ObjectivesComponent.c:813 | TRUE | T-941.4 |
| spectator range 0 = unlimited | Spectator/TBD_SpectatorComponent.c:61-62 (:56-60) | TRUE | T-941.5 |
| link code in public chat | Gamemode/TBD_AdminCommands.c:41; Backend/TBD_IdentityLink.c | TRUE | T-941.6 |
| radio NO_BACKBONE | Radio/TBD_RadioTuner.c:49-51, :142-143; TBD_RadioComponent.c:135 | TRUE | T-941.7 |
| naked vehicles | Gamemode/TBD_SpawnManager.c (crew: T-675.2) | TRUE | T-941.8 |

## 3. Design and wave packing
- .1 arm predicate LOBBY|BRIEFING|SAFE_START, disarm on LIVE; .2 deploy on LOBBY→BRIEFING once per player, DEPLOY disabled while pending, reopen path.
- .3 UI/End/TBD_EndScreen.c + TBD_DebriefScreen.c + two layouts, hooked in TBD_FrameworkManager.c; .4 UI/Hud/TBD_ObjectiveHud.c + layout, replicated state replaces the chat pump.
- .5 default 2000 m, authority clamp; .6 consume-before-broadcast, private replies; .7 script channel table on NO_BACKBONE, actionable warning; .8 fuel full + class cargo, roster override.
Waves: A = .1 .2 .3 .5 .6 .7 (disjoint); B = .4 (after .3), .8 (after .2 and queued T-675.2: TBD_SpawnManager.c).

## 4. Rules every slice encodes
Defect evidence from the anchor lines on main (quoted in the report); perturbation = a deliberate type/name break that turns
`cargo xtask mod compile` red, then restore and `touch`; no `git add -A`, no `git stash`, no `cargo xtask ci ci-local`; `skip:` = FAIL;
no .py/.sh/.mjs committed; agents never merge/push/change status; the gate never stands in for the checklist. Report schema:
pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits.

## Claude Code prompt — T-941.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.1 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_1_plan.md; Gamemode/TBD_SafestartManager.c:240-270,380-420; the stage enum in Core/.
═══ PROBLEM ═══
The shield arms only for SAFE_START (:252-265, deliberate :248-250) while LOBBY auto-deploy puts bodies
in the world ~250 ms in, so players are damageable before go-live.
═══ SHIPPED ═══
Safestart arm/disarm; GoLive with three countdown exits (:389-417, untouched).
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion). Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- Arm for LOBBY, BRIEFING, SAFE_START; disarm once on LIVE; no per-body re-apply.
- The :248-250 comment becomes a dated decision naming T-941.1.
═══ DO ═══
1. Verify on main: read :252-265; quote the armed stage set in the report.
2. Widen the predicate; update the comment.
3. cargo xtask mod compile; cargo xtask mod world-boot.
4. Perturbation: compare the stage to a string literal → compile red; restore, touch, green.
═══ DO NOT ═══
No countdown changes; no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.1
═══ MANUAL ═══
Deploy during LOBBY, throw a grenade at your feet: no damage; after GoLive: damage.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-941.2

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.2 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_2_plan.md; UI/Lobby/TBD_LobbyScreen.c:200-220,480-520; Gamemode/TBD_SpawnManager.c (m_bAutoDeploy, deploy path).
═══ PROBLEM ═══
Auto-deploy fires ~250 ms into LOBBY; the DEPLOY race is acknowledged (:211-212) and gating (:492-507)
closes the screen instead of locking, so double bodies and locked-out slot changes happen.
═══ SHIPPED ═══
Lobby screen, slot claims, ShouldStandDown + IsDeployPending.
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion). Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- Deploy on LOBBY→BRIEFING, once per player id (deployed set); nothing spawns during LOBBY.
- DEPLOY disabled with a reason while pending; reopen from the pause menu despawns then redeploys.
- TBD_SpawnManager.c is shared with T-941.8 and queued T-675.2: separate waves.
═══ DO ═══
1. Verify on main: quote :211-212 and the auto-deploy timing in the report.
2. Move the deploy trigger; add the deployed set; disable the button; add the reopen action; date the decision.
3. cargo xtask mod compile; cargo xtask mod world-boot.
4. Perturbation: mismatched type on the deployed set → compile red; restore, touch, green.
═══ DO NOT ═══
No vehicle spawn changes (T-941.8); no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.2
═══ MANUAL ═══
Claim slots with two players; advance to BRIEFING: one body each; double-click DEPLOY: still one; reopen, change slot: redeployed.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-941.3

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.3 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_3_plan.md; UI/TBD_UILayouts.c; Gamemode/TBD_FrameworkManager.c:1010-1030,1115-1170,1285-1300; UI/layouts/TBD_ScreenShell.layout, TBD_ListRow.layout; Backend/TBD_ResultsReporter.c counters.
═══ PROBLEM ═══
Only SCREEN_SHELL and LIST_ROW are registered (:28,31); the END and DEBRIEF stage hooks exist with no
screen, so a mission ends with a bare stage change.
═══ SHIPPED ═══
Stage hooks; shell + list-row layouts; per-player counters in the results reporter.
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion) + .layout. Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- New UI/End/TBD_EndScreen.c (winner + reason) and TBD_DebriefScreen.c (scoreboard, sortable by kills).
- Two new layouts under UI/layouts/ reusing the shell and list-row widgets.
- Screens open in their stage hook and close on the next stage; never block a transition.
═══ DO ═══
1. Verify on main: quote :28-31 and the four hooks in the report.
2. Write the layouts and the two screens; register in TBD_UILayouts.c; hook in TBD_FrameworkManager.c.
3. cargo xtask mod compile; cargo xtask mod world-boot.
4. Perturbation: missing widget name in TBD_EndScreen.c → compile red; restore, touch, green.
═══ DO NOT ═══
No objective HUD (T-941.4); no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.3
═══ MANUAL ═══
End a mission: banner names winner and reason; DEBRIEF lists every player with kills and deaths; next stage closes both.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-941.4

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.4 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_4_plan.md; Objectives/TBD_ObjectivesComponent.c:20-40,790-830 and every SendPrivateMessage call; UI/End/TBD_EndScreen.c (T-941.3 pattern).
═══ PROBLEM ═══
Objective state and capture progress reach players only as private chat lines (:813), so the chat log
is spammed and the HUD shows nothing.
═══ SHIPPED ═══
Objective registry and capture logic; T-941.3 screen/layout pattern.
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion) + .layout. Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- New UI/Hud/TBD_ObjectiveHud.c + UI/layouts/TBD_ObjectiveHud.layout; layout path via the HUD's own constant.
- State and progress replicated at a bounded tick; the pump is removed; only the objective-complete line stays in chat.
═══ DO ═══
1. Verify on main: quote :813 and the call sites in the report.
2. Write the HUD and layout; replicate state and progress; remove the pump.
3. cargo xtask mod compile; cargo xtask mod world-boot.
4. Perturbation: bind the bar to a missing widget → compile red; restore, touch, green.
═══ DO NOT ═══
No TBD_UILayouts.c edit (T-941.3); no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.4
═══ MANUAL ═══
Enter a capture zone: bar fills; complete it: icon changes; chat shows one completion line and no ticks.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-941.5

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.5 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_5_plan.md; Spectator/TBD_SpectatorComponent.c:40-80 and the range request path.
═══ PROBLEM ═══
m_fHostMaxRangeM (:61-62) defaults to 0 meaning unlimited (:56-60) and client requests are not clamped
on the authority, so spectators can see the whole map.
═══ SHIPPED ═══
Spectator component with a host range attribute.
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion). Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- Default 2000 m; 0 → default, never unlimited; authority clamps every request.
- The :56-60 comment states the new semantics with the T-941.5 date.
═══ DO ═══
1. Verify on main: quote :56-62 in the report.
2. Change the default; add the clamp; reword the comment.
3. cargo xtask mod compile; cargo xtask mod world-boot.
4. Perturbation: clamp against a string literal → compile red; restore, touch, green.
═══ DO NOT ═══
No spectator UI changes; no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.5
═══ MANUAL ═══
Request 10 km on a 2 km server: held at 2 km; unset attribute: 2 km.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-941.6

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.6 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_6_plan.md; Gamemode/TBD_AdminCommands.c:30-60; Backend/TBD_IdentityLink.c; Backend/TBD_PlayerIdentity.c:1-30 (docs).
═══ PROBLEM ═══
`#tbd link <code>` reaches public chat before TBD_AdminCommands.c:41 handles it, so every player sees
the account-link code.
═══ SHIPPED ═══
TBD_IdentityLink link/status flow; T-393 API side; SCR_ChatComponent.SendPrivateMessage path.
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion). Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- The command is consumed before broadcast; echo suppressed; replies private.
- The code goes server→backend and is never echoed; no filtering beyond the command prefix.
═══ DO ═══
1. Verify on main: quote :41 and the IdentityLink handler entry in the report.
2. Consume-and-suppress in TBD_AdminCommands.c; private replies in TBD_IdentityLink.c.
3. cargo xtask mod compile; cargo xtask mod world-boot.
4. Perturbation: misspell the private-message method → compile red; restore, touch, green.
═══ DO NOT ═══
No backend API changes; no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.6
═══ MANUAL ═══
Two clients: one types `#tbd link ABC123`; the other sees nothing; the sender gets a private confirmation; `#tbd link status` is private.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-941.7

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.7 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_7_plan.md; Radio/TBD_RadioTuner.c:40-60,130-160; Radio/TBD_RadioComponent.c:120-150; the radioPlan the mission loader exposes.
═══ PROBLEM ═══
GetBackbone() null sets NO_BACKBONE (:142-143) and radios refuse to tune; the boot warning (:135) does not
say what to add. TBD_Dev_POC.ent lacks RadioManagerEntity.
═══ SHIPPED ═══
Tuner state enum; boot warning; flatten-synthesized radioPlan (flatten.rs:1927-1960).
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion). Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- Operator deferral: RadioManagerEntity world edit — deferred by operator 2026-09-04. No .ent edit here.
- NO_BACKBONE → script channel table (radioPlan when present, else defaults); tuning works.
- Warning names the world, RadioManagerEntity and the fallback, once per boot.
═══ DO ═══
1. Verify on main: quote :142-143 and :135 in the report.
2. Add the fallback table and tune path; reword the warning.
3. cargo xtask mod compile; cargo xtask mod world-boot (TBD_Dev_POC).
4. Perturbation: wrong return type from the fallback lookup → compile red; restore, touch, green.
═══ DO NOT ═══
No world/.ent edits; no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.7
═══ MANUAL ═══
Two players tune the same fallback channel and hear each other; one warning line. Operator: Workbench → TBD_Dev_POC.ent → add RadioManagerEntity → save → boot.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-941.8

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-941.8 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-941_8_plan.md; Gamemode/TBD_SpawnManager.c vehicle spawn path (post T-941.2 and T-675.2); the roster fields T-675.2 reads.
═══ PROBLEM ═══
Vehicles spawn with engine-default fuel and no cargo; crews come from T-675.2 but the vehicle is
unusable until someone refuels and stocks it.
═══ SHIPPED ═══
T-675.2 roster read + crew seats; T-941.2 deploy path.
═══ LANGUAGE GATE ═══
Enforce Script (Enfusion). Operator: agents may edit the Enfusion mod scripts; gate = cargo xtask mod compile; in-game behaviour goes on a human checklist.
═══ LOCKED ═══
- Fuel full; cargo from a script-side default table keyed by vehicle class; roster fuel/inventory fields override.
- Missing cargo prefab → one warning naming it; the vehicle still spawns.
═══ DO ═══
1. Verify on main: quote the vehicle spawn path in the report.
2. Add fuel and cargo application with the default table.
3. cargo xtask mod compile; cargo xtask mod world-boot.
4. Perturbation: misspell the fuel API → compile red; restore, touch, green.
═══ DO NOT ═══
No crew or roster-read changes (T-675.2); no files outside owns.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask mod world-boot ; cargo xtask platform wave gate --slice T-941.8
═══ MANUAL ═══
Spawn a roster vehicle: fuel full, class cargo present; unknown cargo prefab: one warning, vehicle present.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```
