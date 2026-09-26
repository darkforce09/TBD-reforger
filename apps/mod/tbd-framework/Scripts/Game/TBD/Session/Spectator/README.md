# Spectator

Where a dead player spends the rest of the [event](/documentation_v2/glossary/a_to_f.md#event) under one
life: a free, follow or first-person camera, a roster of who may be watched, and a server-side
streaming host that keeps the world around the camera loaded.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/
├── TBD_SpectatorCamera.c      `TBD_SpectatorCamera`: free flight, follow orbit and first person
├── TBD_SpectatorComponent.c   game mode component that starts and stops the controller and host
├── TBD_SpectatorController.c  client lifecycle: enters on death, leaves on a living body, follows
├── TBD_SpectatorHost.c        server streaming hosts, and the camera-position RPC that moves them
├── TBD_SpectatorHostEntity.c  the inert, damage-free entity a dead player possesses
├── TBD_SpectatorTargets.c     who a spectator may watch, grouped by faction and group
└── UI/                        the roster screen over the live camera
```

## How it works

```text
server: character dies -> TBD_SpawnManager marks the life spent
client: TBD_SpectatorController.Tick (every 250 ms) sees no living controlled entity
          -> Enter: spawns TBD_SpectatorCamera at the corpse, opens the TBD_Spectator screen
          -> every second tick: TBD_ReportSpectatorCamera(position)  --RPC-->  server
server: TBD_SpectatorHost gives the dead player a TBD_SpectatorHostEntity to possess
          and moves it to the reported position, clamped to the range from the death spot
client: a later tick sees a living body again (an admin respawn) -> Leave
```

`TBD_SpectatorComponent`, on `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, starts
`TBD_SpectatorHost` on the authority and, on a machine with a workspace, `TBD_SpectatorController`
2 s after init; its `OnDelete` shuts both down. Its attributes:

| Attribute | Default | Effect |
|---|---|---|
| `m_bStreamingHost` | on | gives dead players a streaming host; off leaves streaming anchored to the corpse |
| `m_sHostPrefab` | empty | an optional host prefab; empty spawns `TBD_SpectatorHostEntity` by type name |
| `m_fHostMaxRangeM` | 2000 | how far, in metres, a host may move from the death position; 0 or less means 2000 |

The controller polls instead of hooking a death event, because "do I control a living character"
is a question the client answers itself; a player who reconnects with a spent life and no body
enters after `NO_BODY_GRACE_MS` (20 s). The mission's `spectatorPolicy`, which `TBD_FrameworkManager`
replicates, decides the rest: `none` keeps the death view and no camera, `own_side_delayed_60s`
waits `OWN_SIDE_DELAY_MS` (60 s) and restricts the roster to the viewer's side, and `free` shows
every side. `TBD_SpectatorTargets` holds that policy: own side by default, failing closed to an
empty list when the viewer's faction cannot be resolved, and latching the faction the first time it
resolves. The restriction runs on the client, so it is a discipline measure, not a security
boundary; the engine's replication range is the hard limit.

Streaming follows the controlled entity, not the camera, which is why the host exists.
`TBD_SpectatorHost` refuses to possess anything that is a `ChimeraCharacter` or carries a damage or
character controller component, so a host can never be killed or become a playable body. A
one-second reconcile tick gives each dead connected player a host and retires every record whose
connection epoch has moved on, since player ids are recycled. `TBD_SpawnManager` releases the host
before a deployment and on disconnect.

## Authority

- Server: `TBD_SpectatorHost` (`@authority server` on every method): creating, possessing, moving,
  validating and releasing hosts; `TBD_SpectatorComponent` starts it only when `RplSession.Mode()`
  is not `RplMode.Client`.
- Client: `TBD_SpectatorController`, `TBD_SpectatorCamera`, `TBD_SpectatorTargets` and the roster
  screen, started only where `GetGame().GetWorkspace()` exists. These files carry no authority tag.
- Owner: `TBD_ReportSpectatorCamera` on the owning client's `SCR_PlayerController` sends the camera
  position; on a listen host it calls `TBD_SpectatorHost.MoveTo` directly.
- RPCs: `TBD_RpcAsk_SpectatorHostAt(vector)` on the modded `SCR_PlayerController`, Unreliable to
  the Server; the server moves the host of `GetPlayerId()`, never a player the client names.
- Replicated properties: none here; the spectator policy arrives through `TBD_FrameworkManager`.

## Boundaries

- Depends on: `TBD_SpawnManager` (the spent life, `ReleaseFor` calls) in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_FrameworkManager` (the
  spectator policy) in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`;
  `TBD_MenuStack` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; the input context
  `apps/mod/tbd-framework/Configs/System/ActionContext/TBD_SpectatorContext.conf` and vanilla's
  `ManualCameraContext`; the engine's `SCR_CameraBase`, `CameraManager` and
  `SCR_PlayerController.SetPossessedEntity`.
- Used by: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_SpectatorComponent`; `TBD_SpawnManager` (`TBD_SpectatorHost.ReleaseFor`);
  `TBD_MissionLoader` (`TBD_SpectatorTargets.SetFactionRestricted`); `TBD_PreSlotCamera` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/` (`TBD_SpectatorController.IsActive`);
  `TBD_FrameworkManager`, whose roll call checks the component.
- Rules: a host is never a character, never damageable and never a path back into the world
  (`TBD_SpawnManager.AdminRespawn` is the only one); the server trusts no client position beyond its
  own clamps; statics are cleared on shutdown because they outlive a world; lines added stay ASCII
  and `cargo xtask mod compile` checks that the scripts compile, while spectating is checked by hand
  on a dedicated server.

## Related documentation

- [Spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md)
  — the spectator as built, its policies and controls, and the design target
