# UI/Common

Handlers for the shared sub-layouts in `UI/layouts/Common/` and `UI/layouts/Session/Shared/`.
Pure view binding: no screen knowledge, no network access. Each class binds its layout by widget
name (contract tables: `UI/layouts/Common/README.md`, `UI/layouts/Session/Shared/README.md`) and
exposes a small API plus, where it has an output, one `ScriptInvoker`.

| File | Class(es) | Layout | Output invoker |
|---|---|---|---|
| `TBD_PanelComponent.c` | `TBD_PanelComponent` | `TBD_Panel` | — |
| `TBD_ChipComponent.c` | `TBD_ChipComponent` | `TBD_Chip` | — |
| `TBD_SearchBoxComponent.c` | `TBD_SearchBoxComponent` | `TBD_SearchBox` | `GetOnChanged()(box, query)` |
| `TBD_TabStripComponent.c` | `TBD_NavItemData`, `TBD_NavItemComponent`, `TBD_TabStripComponent` | `TBD_NavItem`, `TBD_TabStrip` | `GetOnSelected()(strip, index)` |
| `TBD_KeyValueRowComponent.c` | `TBD_KeyValueRowComponent` | `TBD_KeyValueRow` | — |
| `TBD_DropdownComponent.c` | `TBD_DropdownItem`, `TBD_DropdownMenuBridge`, `TBD_DropdownComponent` | `TBD_Dropdown`, `TBD_DropdownMenu` | `GetOnChanged()(dropdown, tag)` |
| `TBD_SessionTopBar.c` | `TBD_ESessionTab`, `TBD_SessionIdentity`, `TBD_SessionTopBar` | `Session/Shared/TBD_SessionTopBar` | `GetOnTabSelected()(bar, tab)` |
| `TBD_SessionBottomBar.c` | `TBD_SessionBottomBar` | `Session/Shared/TBD_SessionBottomBar` | `GetOnAction()(bar, id)` |

Both bars are rounded-xl cards (`MountRounded` on `BarBorder`/`BarBG` at `RADIUS_PANEL`); the top bar's identity and player-count boxes are rounded-lg (`RADIUS_ROW`) on the bar fill — the mockup's `bg-[#151b2b] border-[#1e293b]`.

### Conventions
- Interactive handlers (`TBD_NavItemComponent`, `TBD_DropdownComponent`) derive from
  `TBD_UIInteractive`: hover = focus, one click = the action, `Repaint()` reads `TBD_UITheme`.
- Every chrome handler has `SetGround(opaqueArgb)` — the colour it sits on — and paints through
  `TBD_UITheme.PaintOver` so translucent tokens are flattened in sRGB (see the colour law in
  `TBD_UITheme.c`). `TBD_PanelComponent.GetGround()` returns the panel's composited fill;
  `TBD_TabStripComponent.GetItemGround()` the strip's. Chip mounts take the ground as a fourth
  argument (`TBD_ChipComponent.Mount(dock, text, tint, ground)`).
- Every `*Border` / `*BG` dock is filled with a rounded shape at attach
  (`TBD_UILayouts.MountRounded`); `TBD_ChipComponent.SetPill(true)` swaps the tag radius for the
  fully round pill.
- Static `Mount(...)` helpers (`TBD_ChipComponent.Mount`, `TBD_KeyValueRowComponent.Mount`,
  `TBD_DropdownComponent.Mount`) create the layout into a dock and return the handler in one call.
- A handler sits on its layout root; child events (`OnChange` from an EditBox, `OnClick` from a
  clear button) arrive at the root and are matched by widget identity, the way vanilla does it.
- Colour comes from `TBD_UITheme` + `TBD_EUITint`; icons from `TBD_UIIcons` keys.

### Call Flow & Contracts
Instantiated by the layout hierarchy (the `components {}` block of the layout) or mounted at runtime
by `Session/*/UI` screens and `TBD_DockScreen`, which bind model data to widgets through these APIs.
