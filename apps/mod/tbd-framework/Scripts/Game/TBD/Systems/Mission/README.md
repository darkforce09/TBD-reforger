# Mission loading pipeline

Brings the [mission](/documentation_v2/glossary/g_to_m.md#mission) deployed to this server into the running
game: loads and verifies its artifact, parses and validates the document, reads the
[event](/documentation_v2/glossary/a_to_f.md#event) roster, and applies the authored states, environment
and placement to the world. Every other Systems folder reads the loaded document from here.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/
├── Data/        slot and vehicle structs, and the readers for entity, vehicle, gadget and parameter state
├── Ingestion/   environment, weather timeline and spawn placement scatter applied to the world
└── Loaders/     the deployed artifact's load and verification, the parse and validator, the event roster
```

## How it works

`Loaders/` runs first, once per world: `TBD_MissionLoader.BeginLoad` starts the boot sequence in
`TBD_DeployedMission`, which ends in `TBD_MissionLoader.LoadDocument` once the artifact's SHA-256
matches. The parse binds the primary structs, including those in `Data/`; `TBD_MissionValidator`
blocks the load on any error; a valid document then arms the readers in `Data/` and `Ingestion/`
(`TBD_EnvironmentReader.Apply`, `TBD_GadgetFlags.Bind`, `TBD_MissionParams.Resolve`). The rest
apply later, from `TBD_SpawnManager` while slot bodies materialize and spawn, and from the game
mode's tick while the round is live.

Two rules hold across the three folders:

- Readers of keys the primary structs do not declare run their own `JsonLoadContext` pass over
  `TBD_MissionLoader.GetRawJson()` into wire structs beside the code that applies them.
- `JsonLoadContext` allocates a nested `ref` field even when its key is absent, so presence is a
  sentinel (`ABSENT`, `Y_ABSENT`, `INDEX_ABSENT`), an empty-string test or a `Count()` test, never
  a null test.

## Authority

- Server: everything. The load path starts from `TBD_FrameworkManager.OnPostInit`, which returns on
  `RplMode.Client`, and every reader and runtime is guarded or called on the server.
- Client: nothing; clients never hold or parse the mission document.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the platform bridge in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`
  (`TBD_GameRuntimeHttp`, `TBD_LoadedArtifactReport`); `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`
  (hashing, logging, the prefab registry); `TBD_FrameworkManager` (the stage); `TBD_SpawnManager`;
  the engine's weather, damage, fuel, weapon and gadget classes; over HTTP, the
  [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) deployment, artifact and roster routes;
  the wire shape in `contracts_v2/definitions/mission.schema.json`.
- Used by: the stage machine in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; the services
  under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the platform reports in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; and the other Systems folders, which read the
  document through `TBD_MissionLoader`.
- Rules: nothing loads before the artifact's SHA-256 matches, and there is no default mission; the
  document is parsed and held on the server only; wire field names are the JSON keys, each struct
  tagged `@contract` (`cargo xtask schema citations`); lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.
