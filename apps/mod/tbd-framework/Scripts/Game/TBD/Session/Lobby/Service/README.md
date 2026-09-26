# Lobby roster builder and wire

The server side of the lobby: it builds the roster one player sees from the spawn manager's own
slot roster, carries out that player's claim, release and deploy, flattens the roster to one string
for the reply, and proves once per process that the string keeps every field.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/Service/
├── TBD_LobbyRoster.c                the roster models: sides, squads, seats, own seat and verdict
├── TBD_LobbyRosterWire.c            the lobby record set: `Serialise` and `Parse`
├── TBD_LobbyRosterWireSelfCheck.c   the once-per-process round trip and orphan-row check
└── TBD_LobbyService.c               builds one player's roster; claim, release and deploy verdicts
```

## How it works

`TBD_LobbyService.BuildForPlayer` asks the engine's `PlayerManager` whether the player already controls a
body (set before any early return, so an unavailable roster still carries it), then parses the six
tab-separated columns of `TBD_SpawnManager.BuildSlotRoster` (slot key, faction, squad, role, state
`OPEN`, `HELD` or `DEAD`, holder id) into a `TBD_LobbyRoster` with holder and faction display
names (`TBD_MissionFactionNames`) and the reader's own seat marked. A row without exactly six
columns, or whose key a newline would rewrite, is skipped with a WARNING. Both sides are sent in
full: a player picks a side from this roster, unlike the side-scoped briefing.

`ApplyClaim`, `ApplyRelease` and `ApplyDeploy` call `TBD_SpawnManager.ClaimSlot`, `ReleaseSlot` and
`DeployPlayerEx`, which own the rules, and turn the result into a sentence; a refused claim names
who got there first. A deploy is accepted only for `DEPLOYED` or `ALREADY`; `AUTHORIZING` and
`UNAUTHORIZED` (the TBD platform deciding the seat) get their own sentences.

`TBD_LobbyRosterWire.Serialise` writes one `TBD_WireCodec` record per line (`M`, `V`, `L`, `D`, then
either `X` or every `F` side, `G` squad and `S` seat), at most 600 lines. Counts are never sent:
`TBD_LobbyRoster.Recount` derives them from the seats after each parse and each optimistic edit.
`Parse` skips a malformed record, and a side or squad record it cannot decode drops the rows under
it rather than attaching them to the one above. `TBD_LobbyRosterWireSelfCheck.Run`, armed by
`TBD_LobbyComponent.OnPostInit`, logs `wire self-check PASS` or an ERROR `FAIL` with the measured
`split-empties=` verdict; the zero-player `cargo xtask mod world-boot` fails on the ERROR.

## Authority

- Server: `TBD_LobbyService` (`@authority server` on every method) and
  `TBD_LobbyRosterWire.Serialise`.
- Client: `TBD_LobbyRosterWire.Parse`, from `TBD_LobbyClient.Accept`; the models live on both.
- Owner: nothing here; the RPCs live on the modded `SCR_PlayerController` in the parent folder.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`;
  `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`;
  `TBD_MissionFactionNames` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`;
  `TBD_FrameworkManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`;
  `TBD_WireCodec` and `TBD_Log` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: the modded `SCR_PlayerController`, `TBD_LobbyClient` and `TBD_LobbyComponent` in the
  parent folder.
- Rules: the roster is only ever the parse of `BuildSlotRoster`; every display string passes
  `TBD_WireCodec.Sanitise`; the wire bytes stay what `Parse` and the self-check expect, and the
  self-check passes at boot; lines added stay ASCII and `cargo xtask mod compile` checks that the
  scripts compile.
