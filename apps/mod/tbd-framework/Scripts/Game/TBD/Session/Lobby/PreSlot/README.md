# Pre-slot overlook camera

What a connected player sees while they have no body: a slow orbit over the terrain behind the
slot picker, in place of a black screen. It steps aside as soon as the player controls a body or
the spectator camera takes the view.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/PreSlot/
├── TBD_PreSlotCamera.c      the camera on rails: orbits a focus point, takes no input
├── TBD_PreSlotCameraArm.c   the client lifecycle: poll, grace period, enter and leave
└── TBD_PreSlotComponent.c   game mode component that arms and tears down the lifecycle
```

## How it works

`TBD_PreSlotComponent`, on `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, starts
`TBD_PreSlotCameraArm` 2 s after init when its `m_bPreSlotCamera` attribute is on (the default) and
the machine has a screen (not a dedicated server, and with a workspace); otherwise it logs an INERT
line, a WARNING when switched off, and a player with no body sees black. The arm polls every
250 ms: once the local player has controlled nothing for 3 s it spawns `TBD_PreSlotCamera` by
typename, which orbits the centre of the world's bound box at 800 m radius and 300 m height at
1.5 degrees per second. It hands the view back, preferring the player's own camera, as soon as the
player controls a body, `TBD_SpectatorController` is active, or the world is not a framework world.
The camera reads no mission data or controlled entity, so it needs no replication; a world with a
degenerate bound box logs one WARNING and keeps the player's view.

## Authority

- Server: nothing; a dedicated server logs INERT and arms nothing.
- Client: everything (`@authority client` on the arm and the component's `OnPostInit`).
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_FrameworkManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_SpectatorController` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/`; the engine's `CameraManager`.
- Used by: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_PreSlotComponent`.
- Rules: `Shutdown` runs on every world teardown because statics outlive a world; the view is
  switched away before the camera is deleted; the spectator always wins the view; lines added stay
  ASCII and `cargo xtask mod compile` checks that the scripts compile.
