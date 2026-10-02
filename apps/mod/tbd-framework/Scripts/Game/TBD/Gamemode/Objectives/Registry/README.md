# Objective registry and end triggers

Builds the mission's objectives once per world from its objective zones, their rules and the
typed `objectives[]` rows, finds destroy targets when the round goes live, and answers whether an
objective end trigger has fired.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Registry/
├── TBD_ObjectiveDestroyTargets.c  finds a destroy objective's targets at LIVE and counts what is left
├── TBD_ObjectiveEndConditions.c   all captured by one side, any destroyed, any hold run out
├── TBD_ObjectiveEntityReader.c    reads objectives[] rows (and their wire structs), hands them out by zoneId
├── TBD_ObjectiveRegistry.c        builds the objectives, logs them, answers EvaluateEndTriggers
├── TBD_ObjectiveRuleResolver.c    applies rule defaults and validates each authored rule by kind
├── TBD_ObjectiveRulesReader.c     reads the objective keys of zones[].rules in a second typed pass
└── TBD_ObjectiveTypedBinder.c     joins an objectives[] row onto its objective and reports the join
```

## How it works

`TBD_ObjectivesComponent` calls `TBD_ObjectiveRegistry.Build()` every tick until it succeeds; it
refuses until `TBD_ZoneRegistry` has built from a valid mission, so an empty build is never
cached. `Build` runs the two document readers once, each a `JsonLoadContext` pass that
`TBD_MissionJsonPass` opens over the held mission text: `TBD_ObjectiveRulesReader` binds the
objective keys of `zones[].rules` (joined by index, checked by id), and
`TBD_ObjectiveEntityReader` binds `objectives[]`, drops rows outside the loader's active variant
set, and warns for a second row on one `zoneId`. Then, for every prepared zone whose kind
`KindOf` resolves:

1. `TBD_ObjectiveRuleResolver.ApplyDefaults` fills every rule field.
2. `TBD_ObjectiveTypedBinder.Bind` joins the zone's row, if any: identity, side, label, framing,
   `lock` and `autoLose`, with a warning or note for a missing or disagreeing type, an unknown side
   or `autoLose`, and a side that supplies a hold zone's missing holder.
3. A zone with no usable shape makes the objective inert; otherwise
   `TBD_ObjectiveRuleResolver.Resolve` validates the rules for its kind, and `TBD_ZoneVolume`
   applies the starting owner.

`Build` logs one line per objective, a `built` summary, the declared end triggers that can never
fire (`ReportTriggerCoverage`) and the typed coverage, including every row that bound to nothing.

On the first LIVE tick the component calls `TBD_ObjectiveDestroyTargets.ArmDestroyTargets` for each
destroy objective: `TBD_Registry` resolves `targetAlias`, and `TBD_EntityQuery` collects that
prefab inside the zone and its height band; an unresolved alias or an empty search makes the
objective inert with a reason that names the cause (no `entities[]` row, rows outside the zone, or
a spawn that was skipped or failed) and recounts the usable objectives. `EvaluateDestroy` re-queries
every tick and completes the objective once `RequiredKills()` targets are destroyed or gone.

`TBD_ObjectiveRegistry.EvaluateEndTriggers` checks `all_objectives_captured`,
`objective_destroyed` and `hold_expired` in that order, each only when `winConditions.endOn`
declares it, through `TBD_ObjectiveEndConditions`, and returns the trigger and the winning side.

## Authority

- Server: everything. `Build`, `Read` and the destroy search run only from
  `TBD_ObjectivesComponent`'s server tick; a client holds no mission text, so both readers return
  false and the registry never builds there.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ZoneRegistry`, `TBD_Zone` and `TBD_ZoneVolume` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; `TBD_MissionLoader` (active variants,
  factions, entities, `HasEndTrigger`), `TBD_MissionJsonPass` and `TBD_MissionVariants` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`; `TBD_Registry`, `TBD_Log`,
  `TBD_EntityQuery` and `TBD_DeclaredFactions` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`;
  the `zone`, `zoneRules`, `objective` and `objectiveFraming` definitions in
  `contracts/definitions/mission.schema.json`.
- Used by: `TBD_ObjectivesComponent` and `TBD_ObjectiveProgression` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Runtime/`; `TBD_FrameworkManager`
  (`EvaluateEndTriggers`, `TRIGGER_*`); `TBD_MissionValidator` (`KindOf`, `TYPE_*`, `TRIGGER_*`);
  `TBD_TriggerRuntime` (`GetAll`).
- Rules: the zone is what the runtime enforces and wins every disagreement with a typed row;
  `lock` and `autoLose` are carried and reported, never enforced; a hold objective with no holder
  or length goes inert rather than guessing when the round ends; statics outlive a world, so
  `TBD_ObjectivesComponent.OnDelete` calls `Clear`. `cargo xtask verify destroy-target-diagnostics`
  pins the destroy-target reasons in `TBD_ObjectiveDestroyTargets.c`, and
  `tools/xtask/src/verifications/schemas/tests/checks/staged_golden_tests.rs` requires every
  key the staged 1.3 golden authors under `objectives[]` to be a member of the reader's structs.
