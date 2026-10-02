# Mission objectives and tasks

Runs a [mission](/documentation/glossary/g_to_m.md#mission)'s objectives on the server while the round
is live: capture, hold and destroy objectives on the mission's zones, with per-side task text, the
objective end triggers the round can end on, and the mission's `tasks[]` that follow its editor
triggers.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/
└── Engine/  the objective record, registry, 1 Hz runtime and end triggers, and the task machine
```

## How it works

[`Engine/`](Engine/README.md) is the machinery every objective and task shares: it builds the
objectives once per world from the mission's zones and `objectives[]`, advances capture, hold and
destroy progress once a second while the stage is `LIVE`, sends each player their objective board,
answers the objective end triggers `TBD_FrameworkManager` asks for every 2 s, and moves each task
of `tasks[]` from `assigned` to `succeeded` or `failed`. An objective kind's own behaviour class sits
in a Types folder beside `Engine/`, never inside the engine; the folder exists once the first such
class does, since Git tracks no empty folder.

## Authority

- Server: everything here; clients hold no mission document, so no objective or task builds there.
- Client: the once-a-second task HUD request, armed by `TBD_RuntimeHeartbeat` on a client.
- Owner: each player receives only the board and capture bar built for their own side.
- RPCs: none declared here; the objective and task HUD RPCs live in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`.
- Replicated properties: none.

## Boundaries

- Depends on: the zones and triggers in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`;
  the mission loader under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`;
  `TBD_SpawnManager` and `TBD_FrameworkManager`; the helpers in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the HUD in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; the mission shapes in
  `contracts/definitions/mission.schema.json`.
- Used by: the round-end rules in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Stage/`; `TBD_TriggerRuntime`,
  `TBD_ZoneVolume` and `TBD_MissionValidator`; the HUD and audio scripts; `TBD_RuntimeHeartbeat`;
  and `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_ObjectivesComponent`.
- Rules: objectives advance only while `LIVE`, and containment comes only from `TBD_Zone`; sources
  stay ASCII, and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Objective capture HUD specification](/documentation/mod/tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md)
  — the objective board and capture bar as built, their delivery and design target
