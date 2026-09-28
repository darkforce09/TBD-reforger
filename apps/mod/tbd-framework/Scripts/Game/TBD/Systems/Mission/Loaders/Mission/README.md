# Mission document load and queries

Turns the verified bytes of the deployed [mission](/documentation_v2/glossary/g_to_m.md#mission)
artifact into the document this world runs: parses it, reduces it to its active variants, has it
validated, and answers every query other systems ask of it; the world applier places its entities
and applies its settings when the stage machine puts the document into force. It runs once per
world on the server.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/Mission/
├── TBD_MissionLoader.c          the held document, the load sequence and every query on it
├── TBD_MissionOrbatQuery.c      a squad by callsign and its authored leader seat
├── TBD_MissionVariantFilter.c   the active variant set and the per-collection filter passes
├── TBD_MissionVariantSources.c  the override file, default flags, gate skeleton and crew ids read raw
└── TBD_MissionWorldApplier.c    entities[] placed, spectator policy, environment and gadget hook applied
```

## How it works

`TBD_MissionLoader.BeginLoad` clears the previous world's document and starts
`TBD_DeployedMission`, which calls `LoadDocument` with the artifact bytes once their SHA-256
matches. `LoadDocument` refuses a document over `MISSION_FILE_MAX_BYTES` (8 MiB, equal to
`x-tbd-missionFileMaxBytes` in `contracts_v2/definitions/mission.schema.json`), then
`ParseMissionJson`:

1. parses the text into `TBD_MissionDocumentStruct` with `JsonLoadContext`;
2. runs `TBD_MissionVariantFilter.Apply`. When the document has a top-level `variants` key, the
   active set is the `activeVariants` of `$profile:TBD_VariantConfig.json` when that file authors
   the key, else the `variants[]` rows marked `default: true`. One pass per collection (zones,
   ORBAT groups, vehicles, entities, slots) keeps the rows whose `variantId` is empty or active; a
   dangling id drops its row with a WARNING, and an excluded vehicle takes its entity twin and crew
   seats with it. `TBD_MissionVariantSources` reads what the typed parse cannot: the override file,
   the `default` flags (an Enforce keyword), the slot and vehicle gates and the crew slot ids;
3. runs `TBD_MissionValidator.Run`, and discards the document on any ERROR;
4. on a valid document, calls `TBD_MissionParams.Resolve`, `TBD_ResultsReporter.Arm` and
   `TBD_IdentityLink.Arm`.

The parse changes nothing in the world. `TBD_LoadingGate` (in
`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Stage/`) polls the load once a
second on the main thread and, once the document is loaded and valid, calls
`TBD_MissionWorldApplier.Apply` before anything else it puts into force. `Apply` spawns
`entities[]` through `TBD_Registry` and records each body with `TBD_MissionVehicleRoster` and
`TBD_EntityState`, applies `settings.spectatorPolicy` through `TBD_SpectatorTargets`, then calls
`TBD_EnvironmentReader.Apply` and `TBD_GadgetFlags.Bind`. The spawns therefore run after the world
has created its entities and never on the engine's REST callback thread, whatever path the bytes
arrived by.

`TBD_MissionLoader` answers every later query: `GetMission`, `GetMissionId`, `GetSlots`,
`GetSlotById`, `GetFactions`, `GetZones`, `GetEntities`, `GetVehicles`, `GetSettings`,
`GetRawJson` (for second-pass readers), `GetActiveVariantIds`, `HasEndTrigger`,
`GetBriefingForFaction` and `GetSpawnZoneForFaction`. `TBD_MissionOrbatQuery` answers the squad
questions (`IsSquadLeader`, a squad by callsign).

## Authority

- Server: everything. `BeginLoad`, `LoadDocument`, `ParseMissionJson` and
  `TBD_MissionWorldApplier.Apply` carry `//! @authority server`, and the path starts from
  `TBD_FrameworkManager.OnPostInit`, which returns on a client.
- Client: nothing; clients never hold or parse the document.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the document classes in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/Document/`;
  `TBD_MissionVariants` and the state readers in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`; `TBD_MissionJsonPass` and
  `TBD_EnvironmentReader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Ingestion/`;
  `TBD_MissionValidator` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/Validation/`;
  `TBD_DeployedMission`; `TBD_Registry`, `TBD_Log`, `TBD_SpectatorTargets`, `TBD_GadgetFlags`,
  `TBD_MissionParams`, `TBD_ResultsReporter` and `TBD_IdentityLink`.
- Used by: `TBD_FrameworkManager` (`BeginLoad`, `IsLoaded`, `IsValid`); `TBD_LoadingGate`
  (`TBD_MissionWorldApplier.Apply`); `TBD_DeployedMission` (`LoadDocument`); the spawn system (the
  slot queries, and `TBD_MissionOrbatQuery.IsSquadLeader` from `TBD_SlotBodyDressing`); and every
  system under `apps/mod/tbd-framework/Scripts/Game/TBD/` that reads the loaded document.
- Rules: the static read API of `TBD_MissionLoader` keeps its names and signatures; nothing is parsed
  over the byte cap, and `MISSION_FILE_MAX_BYTES`, `x-tbd-missionFileMaxBytes` and the
  `artifact_bytes` maximum change together (`cargo xtask verify mission-rest-size-limits`); the
  variant filter runs before validation; the parse changes nothing in the world, and the world
  applier runs only from `TBD_LoadingGate` on the main thread; every static resets per world;
  sources stay ASCII and `cargo xtask mod compile` checks that they compile.
