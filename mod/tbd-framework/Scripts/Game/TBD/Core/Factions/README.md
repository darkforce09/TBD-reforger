# Declared factions

The membership test of a faction key against the loaded mission's `factions[]`.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Core/Factions/
└── TBD_DeclaredFactions.c  whether a key is one of the loaded mission's factions[].key
```

## How it works

`TBD_DeclaredFactions.Exists(key)` walks `TBD_MissionLoader.GetFactions()` and returns true on
the first non-null faction whose `key` matches. An empty key, or no loaded faction list, is never
declared.

## Authority

- Server: nothing of its own; the mission document it reads is held only on the server.
- Client: nothing; on a client the faction list is empty and every key reads undeclared.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` and `TBD_MissionFactionStruct` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`.
- Used by: `TBD_ObjectiveTypedBinder` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Registry/` and
  `TBD_ZoneVolume` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/Volumes/`.
- Rules: lines added stay ASCII; `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Framework core utilities](/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
