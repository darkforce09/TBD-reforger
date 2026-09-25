# UI framework core

The foundation every TBD screen stands on: the menu base class and the screen stack that owns
input and focus, the two screen bases (the dock shell and the list shell), the interactive
button, list and scrollbar primitives, the one registry of layout resources, the colour and
typography tokens, and the icon lookup.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/
├── TBD_DockScreen.c     TBD_DockScreen: base of the dock screens; mounts the shared bars, routes tabs
├── TBD_ListBox.c        TBD_ListBox: the pooled list, rows re-bound not rebuilt; TBD_ListRowData
├── TBD_ListBoxRow.c     TBD_ListBoxRow: one pooled row, an item or a section heading
├── TBD_MenuBase.c       TBD_MenuBase: the ChimeraMenuBase every TBD screen derives from
├── TBD_MenuStack.c      TBD_MenuStack: the screen stack; one per preset, input and focus on top
├── TBD_ShellScreen.c    TBD_ShellScreen: header, one list, one primary action; preset TBD_UIShell
├── TBD_UIButton.c       TBD_UIButton: the primary or quiet button
├── TBD_UIIcons.c        TBD_UIIcons: icon key to the addon's texture or a vanilla imageset quad
├── TBD_UIInteractive.c  TBD_UIInteractive: hover and focus as one highlight, a click as the action
├── TBD_UILayouts.c      TBD_UILayouts: every layout and texture resource, and the create helpers
├── TBD_UIScrollBar.c    TBD_UIScrollBar: the 4 px scrollbar every TBD list wears
└── TBD_UITheme.c        TBD_UITheme: colour tokens, sRGB compositing, type and radius; tint enums
```

## How it works

### Screens and the stack

A TBD screen is a `ChimeraMenuPreset` added by a `modded enum ChimeraMenuPreset` beside its
screen class and declared in `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` with its
layout and class. Screens are opened through `TBD_MenuStack` (`Open`, `Replace`, `Close`,
`CloseAll`), which keeps these invariants:

- a preset is open at most once: `Open` on an open preset returns the live screen;
- every close path, Esc and engine teardown included, reaches `TBD_MenuBase.OnMenuClose` and pops
  the stack, so no entry outlives its screen;
- only the top screen re-arms its input context (`MenuContext`) each frame and gets
  `OnScreenUpdate`, so a covered screen neither takes input nor ticks;
- after every push and pop the new top screen seeds focus through `FocusDefault()`.

Subclasses override the `OnScreen*` hooks, never the `OnMenu*` ones. Two bases sit on
`TBD_MenuBase`:

| Base | Layout | Subclasses |
|---|---|---|
| `TBD_DockScreen` | a shell with `TopDock`, `LeftDock`, `CenterDock`, `RightDock`, `BottomDock` and `OverlayDock` | `TBD_MissionSelectorScreen`, `TBD_LobbyScreen`, `TBD_BriefingScreen` |
| `TBD_ShellScreen` | `apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout`: title, a list, one primary action | `TBD_AdminScreen`, `TBD_SpectatorScreen`; the bare `TBD_UIShell` preset |

`TBD_DockScreen.OnScreenOpen` mounts `TBD_SessionTopBar` and `TBD_SessionBottomBar` into the top
and bottom docks and hides the overlay dock; a subclass calls `super.OnScreenOpen()` first, mounts
its panels with `Mount` or `MountHandler`, and fills `GetScreenTitle`, `GetSessionTab`,
`GetSessionIdentity` and `OnBottomAction`. A top-bar tab replaces the screen with the preset
`PresetForTab` names. `OnScreenClose` unmounts everything, so a subclass calls
`super.OnScreenClose()` last.

### Primitives

- `TBD_UIInteractive`, on a `ButtonWidget` root, folds the widget hooks into one highlighted state
  shared by mouse hover and gamepad focus, fires `OnActivated` on the click itself, and repaints
  through `Repaint()`; `TBD_UIButton` and `TBD_ListBoxRow` derive from it.
- `TBD_ListBox` creates each row once per index from `TBD_ListRow.layout` (its `[Attribute]`) and
  afterwards only re-binds text and colour, hiding surplus rows, so a list refreshed on every
  [slot](/documentation_v2/glossary.md#slot) claim costs no widget churn. A build is `BeginUpdate`,
  `AddSection` and `AddItem`, `EndUpdate`.
- `TBD_UIScrollBar` is mounted by the owner of a scroll widget; it ticks at 30 Hz through the call
  queue, sizes and moves the thumb from the viewport and content, and hides when nothing
  scrolls. The engine's own bar is clipped away by the list layout.

### Resources, colour and icons

- `TBD_UILayouts` names every layout and texture once as `"{GUID}path"`; its header holds the GUID
  block ledger (`7BD1A7000000XXnn`). `Create` returns null without a workspace (a dedicated
  server) and retries by bare path when a GUID does not resolve; `CreateStretched` and
  `CreateHandler` wrap it; `MountRounded` fills a `*Border` or `*BG` frame dock with the
  `TBD_Rounded<N>` layout for radius 5 to 12 (8 for any other value) and leaves a non-frame dock
  square.
- `TBD_UITheme` holds the colour tokens, ported by name and hex from
  `apps/website/frontend/style/aegis.css`, under three laws: tokens are sRGB and reach the engine
  through `Color.FromSRGBA`, never `SetColorInt`; alpha is composited in sRGB by `Over(top, ground)`
  (`Paint` over the panel ground, `PaintOver` over a given ground), and only `PaintAlpha` sends
  real alpha, for surfaces over the 3D world; fonts are set in the layouts, and the `FONT_*`
  constants name their GUIDs. `SelfCheck()` warns once if `Over()` drifts. It also holds the
  `TEXT_*` and `RADIUS_*` ladders and the `TBD_EUITint` and `TBD_EUIState` vocabularies, the only
  place a tint or state becomes a colour.
- `TBD_UIIcons.Load` shows the addon's own icon, `TBD_Icon_<key>_UI.edds` in
  `apps/mod/tbd-framework/UI/Textures/TBD/Icons/`, for the 38 keys `BuildShipped()` lists, addressed
  by path because `s_mTextureGuids` pins no GUID; otherwise a quad of the vanilla
  `icons_wrapperUI-64.imageset` that resolved on a real run; else it hides the slot and warns once.

## Authority

- Server: nothing; on a dedicated server there is no workspace, so `TBD_UILayouts.Create` returns
  null and no screen opens.
- Client: everything; the classes run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's `ChimeraMenuBase`, `MenuManager`, `InputManager` and widget API; the
  layouts under `apps/mod/tbd-framework/UI/layouts/` and the textures under
  `apps/mod/tbd-framework/UI/Textures/`; `TBD_SessionTopBar`, `TBD_SessionBottomBar` and
  `TBD_ESessionTab` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; the preset entries
  that the screen classes under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/` add.
- Used by: every screen and panel under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`, which
  subclass the bases and open screens through `TBD_MenuStack`; the components in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/` and the HUD in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; the menu presets in
  `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`, which name `TBD_ShellScreen`; and the
  layouts that attach `TBD_ListBox`, `TBD_ListBoxRow` and `TBD_UIButton` by class.
- Rules: a layout or texture resource is named once, in `TBD_UILayouts`, and a new GUID block is
  checked against the ledger in its header; a colour is a `TBD_UITheme` token or a `TBD_EUITint`,
  never a literal, and a translucent token is composited over its ground; a moved or new layout is
  invisible to the engine until [Workbench](/documentation_v2/glossary.md#workbench) rewrites
  `resourceDatabase.rdb`, while a new script compiles without it; lines added to a script stay
  ASCII, and `cargo xtask mod compile` checks that the scripts compile, while how a screen looks is
  checked in Workbench.

## Related documentation

- [Mod UI documentation](/documentation_v2/mod/tbd-framework/UI/README.md) — where each screen's
  scripts and layouts go, the dock shell, and the layout rules
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the design methodology the
  interaction and colour rules encode
