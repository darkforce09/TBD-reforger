# Objective record and board text

The runtime record of one mission objective, the three enums that classify it, and the text a
player reads for it on the objective board.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Model/
├── TBD_EObjectiveKind.c     TBD_EObjectiveKind: capture, destroy or hold, or NONE for other zones
├── TBD_EObjectiveOnEmpty.c  TBD_EObjectiveOnEmpty: partial capture progress holds or decays
├── TBD_EObjectiveRole.c     TBD_EObjectiveRole: a viewer reads the attacker, defender or neutral text
├── TBD_Objective.c          TBD_Objective: one objective's rules, progress, owner and framing
└── TBD_ObjectiveText.c      TBD_ObjectiveText: board lines and status text from one side's view
```

## How it works

`TBD_Objective` is plain server state. `TBD_ObjectiveRegistry` fills it at build time: the zone it
lives on (`m_Zone`, a strong reference that owns the shape and the containment test), its kind,
its resolved rules, and, when an `objectives[]` row names its zone, the typed fields (entity id,
task type, side, `lock`, `autoLose`, per-side framing). Every tick `TBD_ObjectivesComponent`
writes the presence sample (`BeginSample`, `AddPresence`), and the objective's kind behaviour,
called through `TBD_ObjectiveProgression`, writes the owner, banked progress, hold clock, contest
state and completion. The behaviours hold no state of their own; every field they read or
write is here.

The role queries decide what each viewer reads. `RoleOf` takes the viewer's side, which callers
resolve from the player's assigned slot: the objective is for `objectives[].side` when that names
a faction, else for `zones[].faction`; the side it is for defends it under `hold` and `defend` (or
an untyped hold zone) and attacks it otherwise, and every other side takes the opposite role.
`TitleFor` and `TaskTextFor` return that side's framing, falling back to the label for the title
and to nothing for the task text. `TBD_ObjectiveText` renders a board line as
`<title> [<status>]`, where the status is `inactive` for an inert objective and otherwise the kind
behaviour's `StatusText`: `DESTROYED` or `intact n/m`, `HELD` or `hold ns left (PAUSED)`, or
`neutral`, `OURS` or `held by <side>` with ` -- CONTESTED` or the partial percentage.

## Authority

- Server: everything; the record is built and advanced only by the server-side registry and
  runtime, and no method here asks where it runs.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Zone` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`;
  `TBD_ObjectiveKindBehaviour` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/`,
  which `TBD_ObjectiveText` asks for each kind's status text.
- Used by: every script under `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`;
  `TBD_ZoneVolume` and `TBD_TriggerRuntime` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`; `TBD_MissionValidator`, which uses
  `TBD_EObjectiveKind`.
- Rules: no geometry here, containment is the zone's; an untyped objective leaves every typed field
  empty and renders exactly its label and status; board text stays ASCII (`--`, never an arrow
  glyph); `tools/commands/schema_tooling/src/tests/schema_checks/side_fallback_tests.rs` runs
  `RoleOf`, `TitleFor`, `TaskTextFor` and `MayOwn` from this source.
