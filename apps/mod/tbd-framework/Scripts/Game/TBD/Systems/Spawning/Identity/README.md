# Player identity and the join door

Who a numeric player id is, whether this host can keep ONE LIFE durable, and what happens when a
player's audit succeeds.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Identity/
├── TBD_SpawnIdentityGate.c  refuses SAFE_START and LIVE while a player has no durable identity; the waiver
├── TBD_SpawnIdentityKeys.c  the bind key of a player id and its mode: BACKEND, NAME-HASH or NUMERIC
└── TBD_SpawnJoinAudit.c     the join door: key, epoch, seat hand-back, verdict line, join deploy
```

## How it works

`TBD_SpawnIdentityKeys.PlayerBindKey` resolves the engine's player identity (a backend uuid on a
registered dedicated server, a name-derived `00bbbddd-` uuid on a listen or hosted server) and
caches it per id, falling back to `player:<id>`, which is a lease on a number. Seats, bodies and
lives are keyed on it. `TBD_SpawnIdentityGate.StageRefusal`, asked by `TBD_FrameworkManager.SetStage`
through `TBD_SpawnManager.StageRefusalFor`, refuses `LOBBY` while the loadout settle refuses or
waits, and refuses `SAFE_START` and `LIVE` while ONE LIFE is on and a connected player resolves to
a NUMERIC key, unless an admin signed the waiver with `IDENTITY_OVERRIDE_PHRASE`.

`TBD_SpawnJoinAudit.OnAudit` runs from `TBD_SpawnManager.OnPlayerAuditSuccess`, the one join hook
that fires on a framework world. It resolves the key afresh, opens a new connection epoch, reports
a NUMERIC late join, hands a returning spent life its departed seat back, logs one `[TBD][JIP]`
verdict line, and schedules the deploy 250 ms later unless the life is spent, the mission's
`flow.jip` refuses, the stage is not deployable, the round is in `LOBBY` or the player has no seat.

## Authority

- Server: everything (`@authority server` on the entry points); the gate answers empty on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `SCR_PlayerIdentityUtils`, `TBD_MissionFlow`, `TBD_Log`, and `TBD_SpawnManager` with
  its other helpers.
- Used by: `TBD_SpawnManager` (`GetIdentity`, `GetIdentityGate`, `GetJoinAudit`, `StageRefusalFor`,
  `AcceptNonDurableIdentity`, `RequireDurableIdentity`, `IdentityStatusLine`), which
  `TBD_FrameworkManager` and `TBD_AdminService` call.
- Rules: identity is read only at or after the audit hook, or at disconnect through the cache; the
  gate scans the players connected at each transition and never latches; a repeat audit is
  swallowed only after an identity key was resolved.
