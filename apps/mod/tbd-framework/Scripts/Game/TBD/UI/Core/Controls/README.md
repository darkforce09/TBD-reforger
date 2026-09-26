# Buttons, lists and scrollbars

The interactive primitives every TBD screen and component is built from: the hover-and-focus base,
the button, the pooled list with its rows, and the custom scrollbar.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Controls/
├── TBD_ListBox.c        TBD_ListBox: the pooled list, rows re-bound not rebuilt; TBD_ListRowData
├── TBD_ListBoxRow.c     TBD_ListBoxRow: one pooled row, an item or a section heading
├── TBD_UIButton.c       TBD_UIButton: the primary or quiet button
├── TBD_UIInteractive.c  TBD_UIInteractive: hover and focus as one highlight, a click as the action
└── TBD_UIScrollBar.c    TBD_UIScrollBar: the 4 px scrollbar every TBD list wears
```

## How it works

`TBD_UIInteractive`, on a `ButtonWidget` root, folds the widget hooks into one highlighted state
shared by mouse hover and gamepad focus and fires `OnActivated` on the click itself;
`TBD_UIButton`, `TBD_ListBoxRow` and the nav and dropdown components derive from it. `TBD_ListBox`
creates a `TBD_ListRow.layout` row once per index and afterwards only re-binds text, tag and
state, hiding surplus rows; a build is `BeginUpdate`, `AddSection` and `AddItem`, `EndUpdate`, and
rows paint from `TBD_UIStateColours`. `TBD_UIScrollBar` is mounted by the owner of a scroll widget
into its `ScrollBarDock`, ticks at 30 Hz through the call queue, sizes and moves the thumb from
the viewport and content, and hides when nothing scrolls.

## Authority

- Server: nothing.
- Client: everything; the controls run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's widget API; `TBD_UILayouts`, `TBD_UITheme` and `TBD_UIStateColours` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/`; the layouts
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_ListRow.layout` and
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_ScrollBar.layout`.
- Used by: the shells in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Screens/`; the components
  in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; the HUD in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; the screens and panels under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the layouts that attach `TBD_ListBox`,
  `TBD_ListBoxRow` and `TBD_UIButton` by class.
- Rules: `TBD_ListBox`, `TBD_ListBoxRow` and `TBD_UIButton` are class names the layouts name; a list
  that refreshes on replication pools its rows; lines added to a script stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile.
