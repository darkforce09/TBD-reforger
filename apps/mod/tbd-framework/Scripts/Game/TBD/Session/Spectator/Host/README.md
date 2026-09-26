# Spectator streaming host

The server half of the spectator: an inert entity a dead player possesses so the engine keeps
streaming the world around their spectator camera, and the camera-position RPC that moves it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/Host/
├── SCR_PlayerController.c          the camera-position RPC on the player controller
├── TBD_SpectatorHost.c             the host registry: arming, reconcile tick, moves and release
├── TBD_SpectatorHostEntity.c       the inert, damage-free entity a dead player possesses
├── TBD_SpectatorHostFactory.c      spawns a host by type name or prefab and refuses any body
├── TBD_SpectatorHostLifecycle.c    one reconcile pass: retire stale hosts, issue missing ones
└── TBD_SpectatorHostRecord.c       what the server remembers about one host
```

## How it works

Streaming follows the controlled entity, not the camera. Under one life a dead player still
controls their corpse, so without a host the world far from the corpse is never sent to them.
`TBD_SpectatorComponent` calls `TBD_SpectatorHost.Start` on the authority, which arms a one-second
reconcile tick. Each tick releases every host on a world that is not a framework world, releases
every host and logs once when `TBD_SpawnManager` is missing, and otherwise runs
`TBD_SpectatorHostLifecycle.Reconcile`.

A reconcile pass first retires every record whose entity is gone, whose connection epoch has moved
on (player ids are recycled; this test precedes the controller test), whose controller is gone,
whose stage is before `SAFE_START`, or whose player is not dead. It then issues a host to each
connected player whose life is spent and who has none: the anchor is the corpse, else the ground
under the assigned slot; `TBD_SpectatorHostFactory.SpawnHostEntity` spawns
`TBD_SpectatorHostEntity` by type name (or the configured `m_sHostPrefab`, falling back to the
type name when it will not load); `IsAcceptableHost` refuses a `ChimeraCharacter` or anything with
a `DamageManagerComponent` or `CharacterControllerComponent`; the controller possesses it with
`SetPossessedEntity`, and the possession is read back before the record is stored. A refused
prefab is dropped for the round; a refused built-in host stands the feature down. A refusal to
issue logs once until a host is issued.

The client sends its camera position through `TBD_ReportSpectatorCamera`; the server handler calls
`TBD_SpectatorHost.MoveTo` for `GetPlayerId()`, which checks the epoch and the spent life, clamps
the position to the world box and the range leash, and skips moves under 1.5 m.
`TBD_SpectatorHost.ReleaseFor` un-possesses only when the controlled entity is the host, and
deletes the host on the next frame.

## Authority

- Server: everything but the RPC sender (`@authority server`); `TBD_SpectatorComponent` starts the
  registry only where `TBD_Authority.IsServer()`.
- Client: nothing.
- Owner: `TBD_ReportSpectatorCamera` on the owning client's `SCR_PlayerController`; on a listen
  host it calls `TBD_SpectatorHost.MoveTo` in place.
- RPCs: `TBD_RpcAsk_SpectatorHostAt(vector)`, Unreliable to the Server; the server moves the host
  of `GetPlayerId()`, never a player the client names.
- Replicated properties: none; the prefab-free host has no `RplComponent` and exists on the server
  only.

## Boundaries

- Depends on: `TBD_SpawnManager` (the spent life, connection epochs, assigned slots) in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_FrameworkManager` (the stage) in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_Authority` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the engine's
  `SCR_PlayerController.SetPossessedEntity`.
- Used by: `TBD_SpectatorComponent` in the parent folder (`Start`, `Shutdown`);
  `TBD_SpectatorHostReporter` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/Controller/`
  (`TBD_ReportSpectatorCamera`); `TBD_SpectatorTargets` in the parent folder
  (`TBD_SpectatorHostEntity.IsHost`); `TBD_DeployExecutor` and `TBD_SpawnDeparture` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/` (`TBD_SpectatorHost.ReleaseFor`).
- Rules: a host is never a character, never damageable and never a path back into the world;
  holding one never clears a spent life, claims a slot or touches a body; the server trusts no
  client position beyond its own clamps; statics are cleared on shutdown because they outlive a
  world; lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.
