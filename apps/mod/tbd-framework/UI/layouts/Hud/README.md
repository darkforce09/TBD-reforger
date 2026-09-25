# In-game HUD layouts

The layout of the objective panel a player sees during live play: the list of objectives and a
capture bar, drawn over the world in the top-left corner. It opens when the server's first
objective snapshot reaches the player and closes when the round leaves the live stage.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Hud/
├── TBD_ObjectiveHud.layout       the objective panel: title, objective list and capture bar
└── TBD_ObjectiveHud.layout.meta  its resource GUID, `{7BD1A70000000A01}`
```

## Format

- File type: an [Enfusion](/documentation_v2/glossary.md#enfusion) widget layout, plain text.
  The root frame `HudRoot` (360 x 320 px, 24 px from the top-left corner) carries the
  `TBD_ObjectiveHud` handler and holds the `Panel` image, the `Title` text, a `List` frame with the
  `TBD_ListBox` component (`Scroll`, `Content`, `EmptyState`), the `CaptureLabel` text and the
  `CaptureBar` frame with its `CaptureTrack` and `CaptureFill` images. `CaptureFill` is 328 px
  wide, the `BAR_WIDTH` the handler scales it by. Its texts carry no `FontProperties` block and
  draw in the engine's default font.
- Resource GUID: `{7BD1A70000000A01}` in the `.meta`, block `0A` of the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`; it never changes.
- Naming: `TBD_<Element>Hud.layout`, one file per HUD element.
- Adding a HUD element: take a free block from the ledger, author the layout and its `.meta`, add
  a `TBD_UILayouts` constant, and commit both files; the game finds a new path only after
  [Workbench](/documentation_v2/glossary.md#workbench) has rewritten `resourceDatabase.rdb`.

## Referenced by

- `TBD_UILayouts.OBJECTIVE_HUD` names the layout by GUID and path.
- `TBD_ObjectiveHud` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/TBD_ObjectiveHud.c` uses
  that constant: `Accept` applies each objective snapshot the server sends to the owning player
  and calls `Open`, which creates the layout on the workspace root; a snapshot with `show` 0 calls
  `Close`. The snapshots come from `TBD_ObjectivesComponent` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`.

## Boundaries

- Depends on: the `TBD_ObjectiveHud` and `TBD_ListBox` classes in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/`, which the layout names as components.
- Used by: `TBD_ObjectiveHud` only.
- Rules: the widget names above are the handler's contract; `CaptureFill`'s width stays equal to
  `TBD_ObjectiveHud.BAR_WIDTH`; the layout and its `.meta` are committed together.

## Related documentation

- [Objective capture HUD specification](/documentation_v2/mod/tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md)
  — the HUD as built, its delivery and design target
