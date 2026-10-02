# Typed mission document

The classes a [mission](/documentation/glossary/g_to_m.md#mission) document is parsed into on the
server: the document root with its header and factions, zones, the ORBAT, briefings, placed
entities, pacing and win conditions, policy settings and named variants. They hold data only; the
loader fills them once per load and every system reads them.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/Document/
├── TBD_MissionBriefingStruct.c      one faction's written orders and their map markers
├── TBD_MissionDocumentStruct.c      the document root, its meta header and its playable factions
├── TBD_MissionEntityStruct.c        one mission-placed world object from entities[]
├── TBD_MissionFlowStruct.c          event pacing (flow) and round-end triggers (winConditions)
├── TBD_MissionOrbatFactionStruct.c  one faction's ORBAT: groups, roles and the squad leader seat
├── TBD_MissionSettingsStruct.c      respawn, spectator and night-vision policy
├── TBD_MissionVariantStruct.c       variants[] rows and the slot and vehicle variant-gate mirror
└── TBD_MissionZoneStruct.c          one zone with its circle or polygon shape and its rules
```

## How it works

`TBD_MissionLoader.ParseMissionJson` reads the verified mission text into
`TBD_MissionDocumentStruct` with one `JsonLoadContext` pass. `JsonLoadContext` binds a JSON key
only to a field of the same name, so every field is named as its key and a key no field declares
is invisible; readers of such keys run their own pass over `TBD_MissionLoader.GetRawJson()`.
`default` is an Enforce keyword, so the per-row `default` flag of `variants[]` has no field and
`TBD_MissionVariantSources` reads it off the raw text; `TBD_VariantGateSkeletonStruct` is the root
of a second pass that reads only the `variantId` of every slot and vehicle row.

`JsonLoadContext` allocates a nested `ref <class>` field even when its key is absent (a
polygon-only zone reads its `circle` as x=0 z=0 r=0), and leaves a scalar at its initializer. So
presence is a content test: a circle radius above 0, an `ABSENT` sentinel on
`TBD_MissionZoneRulesStruct` and `TBD_MissionFlowStruct`, an empty string, or `Count()` on an
array. A `ref array<>` field is null when its key is absent.

## Authority

- Server: everything. The document is parsed only by `TBD_MissionLoader` on the server.
- Client: nothing; clients never hold the document.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionSlotStruct` and `TBD_MissionVehicleStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`, `TBD_MissionEnvironmentStruct`
  and `TBD_MissionParamStruct` beside the readers that apply them, `TBD_MissionRadioPlanStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/`, and the wire shape in
  `contracts/definitions/mission.schema.json`.
- Used by: the loader, variant filter, ORBAT query, world applier and validator in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; the zone registry, volumes and
  play area under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; the objective registry
  and stage machine under `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; the briefing, lobby
  and admin services under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the AI, audio,
  marker and radio systems; `TBD_DeclaredFactions` and `TBD_ResultsReporter`.
- Rules: field names are the JSON keys and never change without the schema; every class carries
  its `@contract` tag (`cargo xtask schema citations`); presence is tested on content, never on a
  null nested object; sources stay ASCII and `cargo xtask mod compile` checks that they compile.
