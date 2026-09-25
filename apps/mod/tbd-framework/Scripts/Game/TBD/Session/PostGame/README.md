# Post-game screens

What players see once a round is decided: the END banner and the DEBRIEF scoreboard, opened by the
stage machine and closed by the next stage.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/
└── UI/  the END banner and the DEBRIEF scoreboard overlays
```

## How it works

`TBD_FrameworkManager` owns the round's stages. On every stage change it opens the END banner on
`END` and the DEBRIEF scoreboard on `DEBRIEF` and closes each on any other stage. Both screens read
what the authority decided from replicated `TBD_FrameworkManager` properties: the winner and the
reason, and a packed board of kills and deaths per player.

## Authority

- Server: only the scoreboard rows, built on the authority by `FillScoreboard` and replicated
  through `TBD_FrameworkManager`.
- Client: both overlays, opened on every machine with a workspace.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none here; the end winner, end reason and packed board are
  `TBD_FrameworkManager` properties.

## Boundaries

- Depends on: `TBD_FrameworkManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/` (the stage, the end result and
  the board); the shared UI library in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`.
- Used by: `TBD_FrameworkManager`, which opens and closes the overlays on `END` and `DEBRIEF`.
- Rules: the overlays are workspace widgets, never menus, so no screen can refuse a stage change;
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [End screen specification](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner's design target
- [Debrief specification](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md)
  — the debrief and after-action review design target
