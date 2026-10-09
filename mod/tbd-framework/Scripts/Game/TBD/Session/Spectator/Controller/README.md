# Spectator controller

The client half of the spectator: when the local player is in spectator, what the camera follows,
the keyboard accelerators, and the camera reports that steer the server's streaming host.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/Controller/
├── TBD_SpectatorController.c      the lifecycle: the 250 ms poll, enter, leave and the policy
├── TBD_SpectatorHostReporter.c    sends the camera position to the server every second poll
├── TBD_SpectatorInputActions.c    the roster, next, previous, view and free accelerators
└── TBD_SpectatorTargeting.c       free flight, follow, first person, cycling and the status line
```

## How it works

`TBD_SpectatorComponent` starts `TBD_SpectatorController` 2 s after init on a machine with a
workspace. Its poll runs every 250 ms: on a world that is not a framework world, or before
`SAFE_START`, it leaves spectator. A living local controlled entity (by
`TBD_SpectatorTargets.IsAlive`, which never counts a streaming host) leaves spectator too. Otherwise
the mission's `spectatorPolicy` decides: `none` stays on the death view, `own_side_delayed_60s`
waits `OWN_SIDE_DELAY_MS` (60 s), `free` enters at once. A player seen alive this round enters on
death; one never seen alive enters after `NO_BODY_GRACE_MS` (20 s). `Enter` spawns
`TBD_SpectatorCamera` by type name behind and above the corpse and opens the roster; `Leave`
restores the player's own camera before it deletes the spectator camera.

While spectating, each poll drops a follow whose target died (`TBD_SpectatorTargeting.ValidateFollow`),
turns camera input off while another TBD screen is on top, and calls
`TBD_SpectatorHostReporter.Report`, which sends the camera position every second poll when it has
moved 2 m or more. `TBD_SpectatorTargeting` keeps the follow as a player id and re-resolves a living
entity on every follow, falling back to free flight. `TBD_SpectatorInputActions` registers
`TBD_SpecRoster`, `TBD_SpecNext`, `TBD_SpecPrev`, `TBD_SpecView` and `TBD_SpecFree`; each is also
a click in the roster.

## Authority

- Server: nothing.
- Client: everything here, started only where `GetGame().GetWorkspace()` exists.
- Owner: nothing here; the report goes through `TBD_ReportSpectatorCamera` in
  `mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/Host/`.
- RPCs: none here.
- Replicated properties: none; the stage and `spectatorPolicy` arrive through
  `TBD_FrameworkManager`.

## Boundaries

- Depends on: `TBD_SpectatorCamera` and `TBD_SpectatorTargets` in the parent folder;
  `TBD_FrameworkManager` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`;
  `TBD_MenuStack` in `mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; the modded
  `SCR_PlayerController` in `mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/Host/`.
- Used by: `TBD_SpectatorComponent` in the parent folder (`Start`, `Shutdown`); `TBD_SpectatorScreen`
  in `mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/` (`TBD_SpectatorTargeting`);
  `TBD_PreSlotCamera` in `mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/`
  (`TBD_SpectatorController.IsActive`).
- Rules: "am I in spectator" is polled from the locally authoritative controlled entity, never a
  replicated death flag; the view is switched away before the camera is deleted; statics are
  cleared on shutdown because they outlive a world; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.
