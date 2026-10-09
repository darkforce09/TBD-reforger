# Objective kind behaviours

Everything that differs between a [mission](/documentation/glossary/g_to_m.md#mission)'s capture,
destroy and hold objectives: one behaviour class per objective kind, the NONE behaviour every
other zone falls back to, and the lookup the objectives engine asks for a kind's behaviour.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/
├── Capture/                      objective_capture: owned by standing on it, all_objectives_captured
├── Destroy/                      objective_destroy: targets to destroy, objective_destroyed
├── HoldUntil/                    objective_hold_until: ground held to a clock, hold_expired
└── TBD_ObjectiveKindBehaviour.c  the hook contract, the NONE behaviour and the lookup by kind
```

## How it works

`TBD_ObjectiveKindBehaviour` is the contract and the NONE behaviour at once: every hook has a
neutral default (a no-op, `false`, or an empty string), and each real kind is a subclass that
overrides the hooks it uses. The behaviours are stateless; every objective's rules, progress and
ownership stay on `TBD_Objective`, which the hooks receive.

### Looking a behaviour up

The lookup builds one shared instance per kind on first use and keeps them in statics:

| Lookup | Key | Answers |
|---|---|---|
| `For(kind)` | `TBD_EObjectiveKind` | the kind's behaviour |
| `ForZoneType(zoneType)` | `zones[].type` | the behaviour whose `ZoneType()` matches |
| `ForEndTrigger(trigger)` | `winConditions.endOn` | the behaviour whose `EndTrigger()` matches |
| `ForTaskType(taskType)` | `objectives[].type` | the behaviour whose `AcceptsTaskType` is true |
| `Count()`, `At(index)` | position | the real kinds in enum order CAPTURE, DESTROY, HOLD_UNTIL |

NONE, an unknown kind, an index out of range and every lookup without a match answer the shared
NONE instance, so a caller never holds null. `Count()` and `At()` fix the order every per-kind
walk uses, which is also the order the end triggers are evaluated in. The statics outlive a world,
so `TBD_ObjectivesComponent.OnDelete` calls `Clear()`, and the next lookup builds them again.

### The hooks

| Hook | The engine asks it | NONE answers |
|---|---|---|
| `Kind`, `ZoneType`, `EndTrigger`, `AcceptsTaskType` | the kind's vocabulary | NONE, empty, false |
| `CountLabel` | the word before the kind's usable count in the registry's `built` and the component's `armed` lines | empty |
| `UndeclaredTriggerNote` | the load note for usable objectives whose end trigger `endOn` does not declare | empty |
| `ApplyDefaults` | after the shared rule defaults | nothing |
| `ResolveRules` | after the rules every kind shares | nothing |
| `SideDefendsByDefault` | the framing of an untyped objective | false |
| `SeedFromSide` | after a typed row binds | nothing |
| `LogRules` | after the typed row's load line | nothing |
| `ApplyStartingOwner` | for a non-empty `startingOwner` | nothing |
| `HasEnded` | for the kind's end trigger | false |
| `OnEnterLive` | once, on the LIVE edge, per usable objective | nothing |
| `Advance` | once per 1 Hz tick, per usable objective; returns the chat completion line | empty |
| `StatusText`, `HudIcon` | for a usable objective's board row and HUD glyph | empty |
| `ClaimsCaptureBar` | whether a player inside is shown the capture bar | false |

### Adding a kind

1. Add the value to `TBD_EObjectiveKind` in
   `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Model/TBD_EObjectiveKind.c`.
2. Add a folder here with a `TBD_ObjectiveKindBehaviour` subclass that overrides the hooks the
   kind uses, keeping each base parameter name, and spells its own zone type, end trigger, task
   vocabulary, count label and undeclared-trigger note.
3. Insert one instance into the lookup in `TBD_ObjectiveKindBehaviour.EnsureBuilt`, at the kind's
   enum position. The win-condition checks list the schema's `endOn` values with the kinds'
   triggers in lookup order around `time_limit` and `faction_eliminated`, so the lookup order
   matches the triggers' relative order in `contracts/definitions/mission.schema.json`.

## Authority

- Server: everything; the hooks run only from the objectives engine's server build and tick.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Objective`, `TBD_EObjectiveKind` and `TBD_EObjectiveOnEmpty` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Model/`;
  `TBD_ObjectiveRegistry` (log channel), `TBD_ObjectiveRuleResolver` (shared rule limits) and
  `TBD_ObjectiveRulesStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Registry/`;
  `TBD_ObjectivesComponent.TICK_SECONDS`; `TBD_ZoneVolume` presence queries in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/Volumes/`; `TBD_Log`, `TBD_Registry`,
  `TBD_EntityQuery` and `TBD_DeclaredFactions` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: the objectives engine in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/`; `TBD_ZoneVolume`; the
  win-condition checks under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/` and
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`.
- Rules: behaviours hold no per-objective state; log channels, keys, formats and the chat and HUD
  strings are the objectives engine's and stay byte-identical; `cargo xtask verify
  destroy-target-diagnostics` pins the destroy-target reasons in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/Destroy/TBD_ObjectiveDestroyTargets.c`;
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Objective capture HUD specification](/documentation/apps/mod/tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md)
  — the objective board and capture bar the status texts and glyphs feed
