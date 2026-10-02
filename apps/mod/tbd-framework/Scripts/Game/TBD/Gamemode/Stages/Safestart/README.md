# Safe start

The warmup shield that keeps everyone unhurt from `LOBBY` until the round goes `LIVE`, because TBD
rounds are one life and a negligent discharge before the start would end somebody's
[event](/documentation/glossary/a_to_f.md#event).

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/Safestart/
├── TBD_SafestartManager.c     game mode component: arm, countdown, lift, status and admin length
├── TBD_SafestartProtection.c  TBD_SafestartProtection: the per-body shield and its restore; TBD_SafestartHold
└── TBD_SafestartWatchdog.c    TBD_SafestartWatchdog: repeats an unverified lift every 5 s and reports it
```

## How it works

`TBD_SafestartManager` is a `SCR_BaseGameModeComponent` on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` and owns one `TBD_SafestartProtection` and
one `TBD_SafestartWatchdog`.

1. Arm (`LOBBY`, `BRIEFING` or `SAFE_START`, once per run): the protection sweeps every player body,
   every AI agent's body and every unpossessed [slot](/documentation/glossary/n_to_z.md#slot) body
   from `TBD_SpawnManager`. Each body gets a `TBD_SafestartHold` recording its damage-handling value
   before the first change, then `EnableDamageHandling(false)`, weapon safety on, and shot and
   grenade handlers that delete the projectile the moment it exists. A re-sweep every 3 s covers
   bodies that appear later.
2. Countdown: on `SAFE_START` only, the replicated `m_iSecondsRemaining` counts down once a second
   from the configured length (300 s by default, 5 to 3600 accepted, set by the mission's
   `flow.safeStartSeconds` or `#tbd safestart <seconds>` through `AdminSetSeconds`). Chat milestones
   go to everyone; at zero, or on `#tbd safestart go`, `GoLive` asks the stage machine for `LIVE`.
3. Lift (any other stage): the armed flag clears first, so a handler that fails to unregister is
   inert; then each held body gets back the value it had, verified by read-back, and a body found
   invulnerable is left so and counted. Anything unverified starts the watchdog, which retries every
   5 s and logs at ERROR until the set is empty.

On each machine with a screen the countdown drives `SCR_PopUpNotification` banners
("SAFESTART -- WEAPONS COLD", "SAFESTART OVER -- WEAPONS LIVE") at the `TBD_ClockText` milestones.
The shield covers characters only, cannot holster a weapon, and leaves melee, falls and vehicle
impacts to the damage switch alone.

## Authority

- Server: the shield, the sweeps, the countdown, the lift and the watchdog; `OnPostInit` stops on a
  client and every mutator carries `@authority server`.
- Client: `OnCountdownReplicated` (`@authority client`) shows the pop-ups; the server calls the same
  helper from `SetCountdown`, since the hook never fires on the authority.
- Owner: nothing.
- RPCs: none.
- Replicated properties: `m_iSecondsRemaining`, the countdown in seconds or `NOT_RUNNING` (-1), with
  the hook `OnCountdownReplicated` (`@replicated m_iSecondsRemaining`).

## Boundaries

- Depends on: `TBD_FrameworkManager` (the stage and `SetStage`) in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_SpawnManager` and
  `TBD_MissionLoader` under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_Log`,
  `TBD_PlayerChat` and `TBD_ClockText` under `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the
  engine's `SCR_CharacterDamageManagerComponent`, `CharacterControllerComponent`,
  `EventHandlerManagerComponent` and `SCR_PopUpNotification`.
- Used by: `TBD_FrameworkManager`, which calls `OnStageChanged` on every transition and refuses
  `SAFE_START` on a world without the component; `TBD_MissionFlowReport`, which sets the authored
  length; `TBD_AdminService` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`; and
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches the component.
- Rules: the lift clears the armed flag before touching any body and hands each body back the value
  it had, never `true` by default; `OnDelete` cancels every poll; lines added to a script stay
  ASCII, and `cargo xtask mod compile` checks that the scripts compile, while whether damage stays
  off is checked in a round.

## Related documentation

- [Safe start HUD specification](/documentation/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md)
  — the safe start notices as built and their design target
