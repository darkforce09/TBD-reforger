# Deployed mission loading and the event roster

Boots every world into the [mission](/documentation_v2/glossary/g_to_m.md#mission) deployed to this server
on the platform: reads the [deployment](/documentation_v2/glossary/a_to_f.md#deployment) in effect, takes
its [artifact](/documentation_v2/glossary/a_to_f.md#artifact) from the local cache or the platform, loads
it only when the SHA-256 of its exact bytes matches, then parses and validates it and reads the
[event](/documentation_v2/glossary/a_to_f.md#event) roster that seats players and authorizes their
deployments.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/
├── Mission/                           the parse, the variant filter, the world applier and every query on the document
├── Validation/                        one pass over the parsed document reporting every error and warning
├── TBD_DeployedMission.c              the boot sequence: deployment read, cache or fetch, verification, load
├── TBD_MissionArtifactCache.c         the verified artifact bytes and their deployment in the server profile
├── TBD_MissionArtifactVerification.c  an artifact's bytes, fetched or cached, hashed and matched to the digest
├── TBD_RosterLoader.c                 the event roster: reserved seats and the platform ids of every slot
└── TBD_RuntimeDeploymentStruct.c      the deployment wire struct and its checks, also the cache identity file
```

## How it works

### Boot sequence

`TBD_FrameworkManager.OnPostInit` calls `TBD_MissionLoader.BeginLoad` once per world on the
server; it clears the previous world's document and starts `TBD_DeployedMission.Begin`.

```text
GET /api/v1/game-runtime/deployment ──404 NO_DEPLOYMENT──> no mission (ERROR); no default mission
        │ TBD_RuntimeDeploymentStruct.Parse: ids present, 64-hex SHA-256, 1..8 MiB
        v
cache identity and bytes match? ──no──> GET /api/v1/game-runtime/artifacts/{artifactId}
        │ yes                                  │ staged in received.json, hashed from the file
        v                                      v
TBD_MissionArtifactVerification: SHA-256 of the bytes == artifact_sha256 ──no──> nothing loads (ERROR), retry
        │ yes (fetched bytes are cached)
        v
TBD_MissionLoader.LoadDocument ──> TBD_LoadedArtifactReport: the runtime session starts, reporting it
        │
        v
stage machine leaves LOADING ──> TBD_RosterLoader.BeginLoad (event roster, when the deployment names an event)
```

A deployment that cannot be read (no [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential),
no answer, a refusal) runs the last verified cached artifact with a WARNING; without one no mission
runs (ERROR). Reads and fetches repeat with backoff from 2 s to 60 s after no answer and every 60 s
otherwise, re-reading the backend config each time. The engine's `RestCallback` exposes no response
header, so the artifact route's entity tag and `x-compile-diagnostics-*` headers are not read; the
digest is computed here with `TBD_Sha256Job`, spread across frames.
`TBD_DeployedMission.GetEventId()`, `GetEventMissionId()` and `GetMissionId()` name the running
deployment for the roster, deployment authorization and the results report.

`TBD_MissionArtifactCache` keeps `$profile:TBD_MissionArtifactCache/document.json` (the exact
bytes), `identity.json` (the deployment) and `received.json` (bytes being verified). It is written
only after a verification succeeds, by the boot sequence and by the `load_mission`
[fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) (`TBD_FleetLoadMissionAction`); a write
removes the identity first and writes it last, so an interrupted write leaves no cached artifact.
A read returns the document as text and as bytes (`FileHandle.ReadArray`), and the bytes are hashed
again before anything loads them.

### Parse and validation

`LoadDocument` in `Mission/` refuses a document over `MISSION_FILE_MAX_BYTES` (8 MiB, the same
ceiling as `x-tbd-missionFileMaxBytes` in `contracts_v2/definitions/mission.schema.json`), parses it
into `TBD_MissionDocumentStruct` (`Data/Document/`), filters it to its active variant set, and runs
`TBD_MissionValidator.Run` in `Validation/`. An ERROR (the mission cannot be played) discards the
document, so the stage machine never leaves `LOADING`; a WARNING (it can be played but may not end)
lets the round run, and `#tbd validate` (`TBD_AdminCommands`) replays the findings. A valid document
is applied to the world and arms the readers and reports; each subfolder's README has the detail.

### Event roster

`TBD_RosterLoader` reads `GET /api/v1/game-runtime/events/{eventId}/roster` (wire version 2) for the
event the running deployment names, and keeps two lookups:

- seating, `armaId -> slotUid` from `assignments`: settles once, on the first answer or at the stage
  machine's roster deadline (`ForceSettle`), and a failed fetch settles it empty (round-robin
  seating); `TBD_SpawnManager` reads it through `GetSlotForIdentity`;
- the slot table, `slotUid -> (orbatSlotId, eventMissionId)` from `slots`: loads for any compiled
  slot, reserved or open (`ResolveSlotBinding`), and `IsSlotTableLoaded()` stays false until it has,
  so `TBD_DeploymentAuthorization` refuses deployments into event seats until then.

A fetch without an answer (network, 5xx, watchdog) repeats with backoff from 2 s to 60 s; a refusal
(401, 403, 404, another wire version, an unreadable body, no machine credential) logs one ERROR per
distinct refusal and repeats every 60 s. A roster counts only when its `missionId` is the running
mission; a matching roster that lists no slot also loads. When a slot uid appears under two event
missions, the first listing wins. `Reset()` starts every world afresh, and an answer to an earlier
world's fetch is dropped.

## Authority

- Server: everything. The deployment, cache, verification and roster classes carry
  `//! @authority server`, as do the entry points in `Mission/` and `Validation/`; the load path
  starts from `TBD_FrameworkManager.OnPostInit`, which returns on `RplMode.Client`.
- Client: nothing; clients never hold or parse the mission document, and receive what they need
  from the lobby, briefing and other services.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on:
  - `TBD_GameRuntimeHttp`, `TBD_LoadedArtifactReport`, `TBD_ResultsReporter`, `TBD_IdentityLink`
    and `TBD_BackendConfig` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`;
  - `TBD_Sha256` and `TBD_Sha256Job` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/Hashing/`,
    `TBD_Log` and `TBD_Registry`;
  - the structs and readers in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/` and
    `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Ingestion/`;
  - over HTTP with the `mod_runtime` credential, the API's [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime)
    routes: `/api/v1/game-runtime/deployment` and `/api/v1/game-runtime/artifacts/{artifactId}`
    (`apps/website/api_v2/src/missions/routes.rs`), and `/api/v1/game-runtime/events/{id}/roster`
    (`apps/website/api_v2/src/operations/routes.rs`);
  - the wire shapes in `contracts_v2/definitions/mission.schema.json`,
    `contracts_v2/definitions/mission-deployment.schema.json` and
    `contracts_v2/definitions/game-runtime-roster.schema.json`.
- Used by: `TBD_FrameworkManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`
  (`BeginLoad`, `IsLoaded`, `IsValid`, the roster settle); `TBD_FleetLoadMissionAction`,
  `TBD_ResultsReporter`, `TBD_IdentityLink` and `TBD_PlayerIdentity` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the objective and stage scripts under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; the admin, briefing, lobby and mission
  selector services under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; and every other
  Systems folder, which reads the loaded document through `TBD_MissionLoader`.
- Rules: nothing reads artifact bytes before their SHA-256 matches, and the cache is written only
  after a match, identity last; there is no default mission; `MISSION_FILE_MAX_BYTES`,
  `x-tbd-missionFileMaxBytes` and the `artifact_bytes` maximum change together; wire field names are
  the JSON keys, each struct with its `@contract` tag (`cargo xtask schema citations`); every static
  resets per world; lines added stay ASCII, and `cargo xtask mod compile` checks that the scripts
  compile, while a boot against the platform is checked on a dedicated server.
