# Dropdown popover

The popover every pre-game screen uses for a choice: a trigger pill that opens a single-select or
multi-select menu floating over the screen, such as the mission browser's Modes, the inspector's
version and the briefing markers panel's plan.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/Dropdown/
├── TBD_DropdownComponent.c   TBD_DropdownComponent: the trigger, the items, the selection and the badge
├── TBD_DropdownItem.c        TBD_DropdownItem: one entry: label, badge, caller tag, check state
├── TBD_DropdownMenu.c        TBD_DropdownMenu: the open menu: layout, pooled rows, placement
└── TBD_DropdownMenuBridge.c  TBD_DropdownMenuBridge: closes the dropdown on a scrim click
```

## How it works

`TBD_DropdownComponent` is the handler on `Common/TBD_Dropdown.layout`, a `TBD_UIInteractive`
button, usually created through its static `Mount(dock, overlayHost, label, multi)`. A screen fills
it with `SetItems(array<ref TBD_DropdownItem>)`; single-select picks the first item when nothing is
chosen. A click on the trigger creates a `TBD_DropdownMenu`, which builds
`Common/TBD_DropdownMenu.layout` into the overlay host (the screen's `OverlayDock`, else the
trigger's parent), finds its `TBD_ListBox`, Select All and Deselect All buttons, mounts a
`TBD_UIScrollBar` and attaches a `TBD_DropdownMenuBridge` to its root. The component subscribes to
the list and buttons, has the menu fill its pooled rows and anchor itself under the trigger in
reference pixels (`DPIUnscale`), and moves gamepad focus to the first row.

A row click in single-select sets the choice and closes the menu; in multi-select it toggles the
check and the menu stays open, and the trigger badge shows the checked count. Either way
`GetOnChanged()(dropdown, tag)` fires; Select All and Deselect All fire it with tag -1. Closing
unsubscribes, destroys the menu, and hides the overlay dock again when nothing else floats in it.

## Authority

- Server: nothing.
- Client: everything; the dropdown runs in the local UI and never replicates.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_UIInteractive`, `TBD_UIButton`, `TBD_ListBox` and `TBD_UIScrollBar` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Controls/`; `TBD_UILayouts` and `TBD_UITheme`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/`; `TBD_ChipComponent` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/Layout/`; the layouts
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_Dropdown.layout` and
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_DropdownMenu.layout`.
- Used by: `TBD_BriefingMarkersPanel`, `TBD_MissionInspectorPanel` and `TBD_ScenarioBrowserPanel`
  under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`, through `Mount` and `SetItems`.
- Rules: the five `[Attribute]` fields stay on `TBD_DropdownComponent`, whose class name the
  layout names; the menu opens only into an overlay host that hides itself while empty; lines
  added to a script stay ASCII, and `cargo xtask mod compile` checks that the scripts compile.
