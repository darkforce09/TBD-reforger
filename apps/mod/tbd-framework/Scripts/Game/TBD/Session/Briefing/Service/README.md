# Briefing builder and wire

The server side of the briefing: it builds the briefing one player may read from the loaded
[mission](/documentation_v2/glossary/g_to_m.md#mission) and that player's slot, flattens it to one
string for the reply, and proves once per process that the string format keeps empty fields.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/Service/
├── TBD_BriefingService.c         builds one player's side-scoped payload; the log channel
├── TBD_BriefingText.c            byte-safe paragraph split, trim, word clip, resource and key names
├── TBD_BriefingWire.c            the briefing record set: `Serialise`, `Parse`, `AdoptOrders`
└── TBD_BriefingWireSelfCheck.c   the once-per-process empty-field round trip and its log line
```

## How it works

`TBD_BriefingService.BuildForPlayer` reads the caller's slot from `TBD_SpawnManager` and the
mission from `TBD_MissionLoader`. With no valid mission or no slot it returns a payload that says
why and nothing else. Otherwise it fills the mission name and terrain, the caller's seat and kit,
their side's orders (split into paragraphs within a 6000-byte, 16-paragraph budget per side; each
cut warns once per faction and field through `TBD_WarnOnce`), their side's ORBAT and zones, and the
shared win condition.

`TBD_BriefingWire.Serialise` writes one `TBD_WireCodec` record per line (`M`, `X`, `S`, `K`, `G`,
`R`, `Z`, `W`, `E`), at most 400 lines; the orders ride beside it as three `array<string>` RPC
parameters that `AdoptOrders` copies onto the parsed payload. `Parse` skips a malformed record, and a
group record it cannot decode drops the role records under it. The first `Serialise` of a process,
or `TBD_BriefingService.SelfCheckWire` from the framework roll-call, runs
`TBD_BriefingWireSelfCheck`, which logs `wire self-check PASS` or `FAIL` with the measured
`split-empties=` verdict.

## Authority

- Server: `TBD_BriefingService` and `TBD_BriefingWire.Serialise` (`@authority server` on
  `BuildForPlayer`); the self-check runs where it is armed, the server in practice.
- Client: `TBD_BriefingWire.Parse` and `AdoptOrders`, on the requesting client.
- Owner: nothing here; the RPCs live on the modded `SCR_PlayerController` in the parent folder.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`;
  `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`;
  `TBD_MissionFactionNames` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`;
  `TBD_WireCodec`, `TBD_WarnOnce` and `TBD_Log` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`;
  `TBD_BriefingPayload` in the parent folder.
- Used by: the modded `SCR_PlayerController` in the parent folder; `TBD_FrameworkManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/` (`SelfCheckWire`).
- Rules: another faction's slots, zones and orders never enter the payload; every authored display
  string passes `TBD_WireCodec.Sanitise`; the wire bytes stay what `Parse` and the self-check
  expect; lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.
