# End and debrief screen layouts

The two overlays that close a round: the END banner with the winning faction and the reason, and
the DEBRIEF scoreboard of kills and deaths per player. Each is a full-screen layout that its
handler opens on the workspace when the stage changes, never through a menu.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/PostGame/
├── TBD_DebriefScreen.layout*  the DEBRIEF scoreboard: title, winner, player list, sort action
└── TBD_EndScreen.layout*      the END banner: title, winner, reason and a status line
```

Each `.layout` sits beside its `.layout.meta`, so every line covers the pair.

## Format

- File type: [Enfusion](/documentation_v2/glossary.md#enfusion) widget layouts (`.layout`), plain
  text, each beside a `.layout.meta` whose `Name` holds
  `{GUID}UI/layouts/Session/PostGame/<file>.layout`. Both follow the shape of
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout`: a `Backdrop` image, a
  `PanelFrame` with the `Panel` image, `Title`, `Subtitle`, a `BackAction` button, `HeaderRule`, a
  `List` frame with the `TBD_ListBox` component (`Scroll`, `Content`, `EmptyState`), `FooterRule`,
  `Status` and a `PrimaryAction` button, both buttons carrying `TBD_UIButton`. The root frame
  carries the screen's handler: `TBD_DebriefScreen` or `TBD_EndScreen`. `TBD_EndScreen.layout`
  adds the `Winner` and `Reason` texts, and its handler hides `List` and `PrimaryAction`; the
  debrief's `PrimaryAction` sorts the list. The texts carry no `FontProperties` block and draw in
  the engine's default font, and the surfaces are square images.
- Resource GUID: `{7BD1A70000000801}` (`TBD_EndScreen`) and `{7BD1A70000000901}`
  (`TBD_DebriefScreen`) in the `.meta` files, blocks `08` and `09` of the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`; they never change.
- Naming: `TBD_<Stage>Screen.layout`, one per post-game stage.
- Adding a layout: take a free block from the ledger, author the layout and its `.meta`, add a
  `TBD_UILayouts` constant, and commit both files; the game finds a new path only after
  [Workbench](/documentation_v2/glossary.md#workbench) has rewritten `resourceDatabase.rdb`.

## Referenced by

- `TBD_UILayouts.END_SCREEN` and `TBD_UILayouts.DEBRIEF_SCREEN` name the layouts by GUID and path.
- `TBD_EndScreen` and `TBD_DebriefScreen` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/` create them on the workspace root
  in `Open` and remove them in `Close`; `TBD_FrameworkManager.ApplyEndScreens` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/` opens the END screen on the
  `END` stage and the DEBRIEF screen on `DEBRIEF`, and closes each on every other stage.

## Boundaries

- Depends on: the `TBD_EndScreen`, `TBD_DebriefScreen`, `TBD_ListBox` and `TBD_UIButton` classes,
  which the layouts name as components.
- Used by: the two post-game handlers only.
- Rules: the widget names above are the handlers' contract; the overlays are workspace widgets, not
  menus, so they cannot block a stage change; a layout and its `.meta` are committed together.

## Related documentation

- [End screen specification](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner as built, how a round ends and the design target
- [Debrief specification](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md)
  — the DEBRIEF scoreboard as built, kill counting and the design target
