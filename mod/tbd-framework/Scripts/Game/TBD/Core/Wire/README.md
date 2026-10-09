# Session wire codec

The one-string record format the lobby, briefing and admin snapshots carry over RPC.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Core/Wire/
└── TBD_WireCodec.c  fields, records, flags, sanitising and the clipped join of the RPC payloads
```

## How it works

A payload is records joined by `LINE_SEP` (newline). A record is a bare kind token followed by
fields, each written `<FIELD_SEP><FIELD_MARK><value>` (tab, dot, value). The marker makes every
field a non-empty token, so an empty value survives a split; `Unmark` strips it and reads a token
of one character or less as empty. `Sanitise` replaces tab, newline and carriage return with
spaces, so authored text or a player name cannot shift a record's fields; it is idempotent.
`Field(value, sanitise)` sanitises by default; the admin snapshot passes false because its rows are
sanitised when built. `Record1` to `Record4` build fixed-width records, `Record(fields, ...)`
builds up to five, `Flag` and `IsSet` carry booleans as `1` and `0`, and
`Join(lines, maxLines, channel, clipWarningFormat)` keeps at most `maxLines` records and logs
one warning when it clips.

## Authority

- Server: nothing of its own; the services that build payloads run on the server.
- Client: nothing of its own; the clients that parse payloads call `Unmark` and `IsSet`.
- Owner: nothing.
- RPCs: none of its own; the lobby, briefing and admin RPCs carry its output.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Log` in `mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: the briefing wire (`TBD_BriefingWire`, `TBD_BriefingWireSelfCheck`,
  `TBD_BriefingService`, `TBD_BriefingText`), the lobby roster wire (`TBD_LobbyRosterWire`,
  `TBD_LobbyRosterWireSelfCheck`, `TBD_LobbyService`), `TBD_AdminSnapshotService` and
  `TBD_DebriefScreen` under `mod/tbd-framework/Scripts/Game/TBD/Session/`; `TBD_MissionFactionNames` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`.
- Rules: the separators and marker never change without every reader and writer changing together;
  a record kind is never authored text; lines added stay ASCII; `cargo xtask mod compile` checks
  that the scripts compile.

## Related documentation

- [Framework core utilities](/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
