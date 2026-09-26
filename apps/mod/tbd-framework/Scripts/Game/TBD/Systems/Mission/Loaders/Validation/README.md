# Mission validation

Checks a freshly parsed [mission](/documentation_v2/glossary/g_to_m.md#mission) document in one pass
and reports every problem at once, so an author fixes a broken mission in one edit. An error
rejects the mission and the server stays in the loading stage; a warning lets the round run. Admins
replay the findings in game with `#tbd validate`.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/Validation/
├── TBD_MissionSlotChecks.c           every slot: identity, faction, kit, position, loadout, squad, heading
├── TBD_MissionStructureChecks.c      schema version, meta, factions, zones, ORBAT parity, faction coverage
├── TBD_MissionUnconsumedKeyCheck.c   authored keys this build drops or does not honour
├── TBD_MissionValidationFindings.c   one run's errors and warnings, their log lines and chat rendering
├── TBD_MissionValidator.c            the entry point: runs every check and keeps the last verdict
├── TBD_MissionWinConditionChecks.c   end triggers: schema values, and whether each one can fire
└── TBD_ValidatorSecondPassStruct.c   the second-pass view: layers, faction tickets and role radio nets
```

## How it works

`TBD_MissionLoader.ParseMissionJson` calls `TBD_MissionValidator.Run` after the variant filter.
`Run` creates a `TBD_MissionValidationFindings` and runs, in order, the schema version, meta and
faction checks, the slot checks (which also count slots per faction), ORBAT parity, faction
coverage, the win conditions, the zones and the unconsumed keys; every check runs whatever the
earlier ones found. Each finding reads `<subject> -- <message>` and is logged as its own
`[TBD][Validate]` line, followed by the verdict line and, on a rejection, a banner. `Run` returns
false on any ERROR, and the loader discards the document.

The severity rule: an ERROR when the mission cannot be played (no faction, a slot with no identity,
side or resolvable kit, a spawn off the terrain, a malformed cargo row, an unknown schema version),
a WARNING when it can be played but may not end. So every end-trigger reachability finding is a
WARNING, except `faction_eliminated` with fewer than two sides holding slots, which is not a PvP
event at all. The objective triggers, zone types and kinds come from `TBD_ObjectiveRegistry` and the
`time_limit` duration rule from `TBD_MissionFlow.ResolveSeconds`, so the validator keeps no copy of
either vocabulary.

`TBD_MissionUnconsumedKeyCheck` warns about `environment` (presence), `settings` values with no
seam (`respawn` other than "none", `nightVision: true`), `layers` aliases, positive
`factions[].tickets` and non-empty `orbat` role `radio` lists. The last three are read by a second
`JsonLoadContext` pass into `TBD_ValidatorSecondPassStruct`; when it cannot parse, presence probes
answer instead. `cargo xtask ci schema-validate` requires its `UNCONSUMED-WARN: <key>` markers and
its wiring from `Run`.

The validator keeps the last run's findings after the document is discarded, and
`BuildReportLines` renders them for `#tbd validate` (`TBD_AdminCommands`), capped at 15 errors and
10 warnings.

## Authority

- Server: everything. `TBD_MissionValidator.Run` carries `//! @authority server`; only the server
  parses mission documents.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the document classes in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/Document/` and
  `TBD_MissionSlotStruct`; `TBD_MissionLoader.GetRawJson`, `TBD_MissionJsonPass`,
  `TBD_LoadoutInventoryUtil.CountGear`, `TBD_Registry.GetAllAliases`, `TBD_ObjectiveRegistry`,
  `TBD_MissionFlow`, `TBD_Log` and the world bound box.
- Used by: `TBD_MissionLoader` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/Mission/` (`Run`);
  `TBD_AdminCommands` and the admin snapshot in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`
  (`HasRun`, `Passed`, the counts and `BuildReportLines`).
- Rules: every check runs to completion; nested objects are tested on content, never on a null
  reference; the unconsumed-key markers and `AddWarning` subjects stay as
  `cargo xtask ci schema-validate` pins them, and the entities truth pin stays for
  `cargo xtask verify destroy-target-diagnostics`; sources stay ASCII and `cargo xtask mod compile`
  checks that they compile.
