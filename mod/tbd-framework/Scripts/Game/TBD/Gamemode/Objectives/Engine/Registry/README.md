# Objective registry and end triggers

Builds the mission's objectives once per world from its objective zones, their rules and the
typed `objectives[]` rows, finds destroy targets when the round goes live, and answers whether an
objective end trigger has fired.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Registry/
├── TBD_ObjectiveEntityReader.c    reads objectives[] rows (and their wire structs), hands them out by zoneId
├── TBD_ObjectiveRegistry.c        builds the objectives, logs them, answers EvaluateEndTriggers
├── TBD_ObjectiveRuleResolver.c    applies the shared rule defaults and rules, then hands the kind's to its behaviour
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
set, and warns for a second row on one `zoneId`. Then, for every prepared zone whose type
`TBD_ObjectiveKindBehaviour.ForZoneType` resolves to a real kind behaviour (the kind behaviours
live in
[`Types/`](/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/README.md)):

1. `TBD_ObjectiveRuleResolver.ApplyDefaults` fills every rule field, then the behaviour's
   `ApplyDefaults` writes the kind's own defaults; the behaviour's `SideDefendsByDefault` says
   whether the zone's side defends it.
2. `TBD_ObjectiveTypedBinder.Bind` joins the zone's row, if any: identity, side, label, framing,
   `lock` and `autoLose`, with a warning or note for a missing, unknown (`ForTaskType` finds no
   kind) or disagreeing type, an unknown side or `autoLose`; the behaviour's `SeedFromSide` lets
   the side supply what the zone left empty (a hold zone's holder).
3. A zone with no usable shape makes the objective inert; otherwise
   `TBD_ObjectiveRuleResolver.Resolve` validates the shared rules and the behaviour's
   `ResolveRules` the kind's, and `TBD_ZoneVolume` applies the starting owner.

Usable objectives are counted per kind (`UsableCountOf`; `UsableCountsSummary` prints every
kind's `CountLabel` and count in the lookup's order). `Build` logs one line per objective (the
common fields, the typed row, then the behaviour's `LogRules`), a `built` summary, the declared
end triggers that can never fire and, through each behaviour's `UndeclaredTriggerNote`, the kinds
whose trigger is not declared (`ReportTriggerCoverage`), and the typed coverage, including every
row that bound to nothing.

A destroy objective's targets are found on the first LIVE tick by `TBD_ObjectiveDestroyTargets`,
which lives with the destroy behaviour in
[`Types/Destroy/`](/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/Destroy/README.md);
when its search makes an objective inert it calls `TBD_ObjectiveRegistry.RecountUsable`.

`TBD_ObjectiveRegistry.EvaluateEndTriggers` walks the kind behaviours in the lookup's order
(capture, destroy, hold), asks each behaviour's `HasEnded` only when `winConditions.endOn` declares
its `EndTrigger`, and returns the first trigger that fired and the winning side.

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
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; `TBD_MissionLoader` (active variants,
  factions, entities, `HasEndTrigger`), `TBD_MissionJsonPass` and `TBD_MissionVariants` under
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`; `TBD_Registry`, `TBD_Log`,
  `TBD_EntityQuery` and `TBD_DeclaredFactions` in `mod/tbd-framework/Scripts/Game/TBD/Core/`;
  `TBD_ObjectiveKindBehaviour` and its three kind behaviours in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/`, which own every per-kind
  rule, log line, vocabulary entry and end condition; the `zone`, `zoneRules`, `objective` and
  `objectiveFraming` definitions in `contracts/definitions/mission.schema.json`.
- Used by: `TBD_ObjectivesComponent` (`Build`, `UsableCountOf`, `UsableCountsSummary`) in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Runtime/`;
  `TBD_FactionElimination` and `TBD_EndBanner` (`EvaluateEndTriggers`); `TBD_ObjectiveDestroyTargets` (`RecountUsable`); the kind behaviours
  (`CH`, the rule resolver's shared limits); `TBD_TriggerRuntime` (`GetAll`).
- Rules: the zone is what the runtime enforces and wins every disagreement with a typed row;
  `lock` and `autoLose` are carried and reported, never enforced; a hold objective with no holder
  or length goes inert rather than guessing when the round ends; statics outlive a world, so
  `TBD_ObjectivesComponent.OnDelete` calls `Clear`. `cargo xtask verify destroy-target-diagnostics`
  pins the destroy-target reasons in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/Destroy/TBD_ObjectiveDestroyTargets.c`, and
  `tools/commands/schema_tooling/src/tests/schema_checks/staged_golden_tests.rs` requires every
  key the staged 1.3 golden authors under `objectives[]` to be a member of the reader's structs.
