# Destroy objectives

The `objective_destroy` kind: prefabs inside a zone that a side must destroy, found when the round
goes live, which completes the objective and ends the round on `objective_destroyed`.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/Destroy/
├── TBD_ObjectiveDestroyBehaviour.c  the destroy hooks: rules, load log, end condition, LIVE edge, tick, text
└── TBD_ObjectiveDestroyTargets.c    finds a destroy objective's targets at LIVE and counts what is left
```

## How it works

`TBD_ObjectiveDestroyBehaviour` answers the destroy hooks of `TBD_ObjectiveKindBehaviour`. Its task
type is `destroy`. `ResolveRules` makes the objective inert without a `rules.targetAlias`, takes
`targetCount`, and warns when the zone names no `faction` (the side told to destroy it, and the
winner). A `startingOwner` means nothing here and is ignored.

On the LIVE edge `OnEnterLive` calls `TBD_ObjectiveDestroyTargets.ArmDestroyTargets` once per
objective: a search at load could run before `TBD_MissionWorldApplier` and other subsystems place
the targets. `TBD_Registry` resolves `targetAlias`, and `TBD_EntityQuery` collects that prefab
inside the zone and its height band; an unresolved alias or an empty search makes the objective
inert with a reason that names the cause (no `entities[]` row, rows outside the zone, or a spawn
that was skipped or failed) and calls `TBD_ObjectiveRegistry.RecountUsable`.

```text
LIVE edge ──▶ ArmDestroyTargets ──▶ targets found | inert with the reason
each tick ──▶ Advance ──▶ EvaluateDestroy re-queries the zone
          ──▶ RequiredKills() gone ──▶ complete ──▶ "TBD: <name> has been DESTROYED."
```

The DESTROYED line goes back to the engine, which puts it in chat. `HasEnded` fires on any usable
complete destroy objective and names its zone's `faction`, which may be empty. The board status
reads `DESTROYED` or `intact <destroyed>/<required>`; the HUD glyph is `#`.

## Authority

- Server: everything; the target search and count run from the objectives engine's server tick.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ObjectiveKindBehaviour` in the parent folder; `TBD_Objective` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Model/`;
  `TBD_ObjectiveRegistry` (log channel, `RecountUsable`) and `TBD_ObjectiveRulesStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Registry/`; `TBD_Registry`,
  `TBD_EntityQuery` and `TBD_Log` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`;
  `TBD_ZoneVolume` and the mission loader's `entities[]` rows.
- Used by: the lookup in `TBD_ObjectiveKindBehaviour`, through which the objectives engine reaches
  it; the objectives engine also calls `TBD_ObjectiveDestroyTargets` directly.
- Rules: targets are re-queried every evaluation rather than cached as handles, so a deleted
  target and a destroyed one read the same; `cargo xtask verify destroy-target-diagnostics` pins
  the signatures of `ArmDestroyTargets` and `DiagnoseEmptyDestroyTargets` and the inert reasons in
  `TBD_ObjectiveDestroyTargets.c`; the log and chat strings stay byte-identical.

## Related documentation

- [Destroy-target diagnostics gate](/tools/checks/mod_script_checks/src/destroy_target_diagnostics/README.md)
  — what the gate pins in the target search and why
