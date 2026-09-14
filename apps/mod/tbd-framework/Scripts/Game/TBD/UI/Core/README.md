# UI/Core

Foundational UI framework primitives, layout registry, Aegis styling system, and navigation stacks.

### Roles & Responsibilities
- `TBD_UILayouts.c`: Single source of truth registering all `.layout` resource GUIDs and relative
  fallback paths, plus `Create` / `CreateStretched` / `CreateHandler` / `Clear` and
  `MountRounded(dock, radius)` (fills a `*Border` / `*BG` frame dock with the nearest
  `TBD_Rounded*` shape; no-op on a dock that is still an image). Its header holds the GUID block
  ledger.
- `TBD_UITheme.c`: Styling layer porting Aegis design tokens (colors, typography, margins) from
  platform CSS to Enfusion. Three laws, all measured on the rebuilt selector (2026-09-12):
  1. tokens are **sRGB**; `Colour()` converts through `Color.FromSRGBA` because the engine's int
     colour APIs are linear (`SetColorInt` rendered every token ~2.2x too bright);
  2. **alpha is composited in sRGB by `Over(top, ground)`**, never by the engine (which blends
     linear and turned white/3 % headers into `#343434` bands and 25 % faction tints into solid
     navy). `Paint` flattens over `PanelGround()`, `PaintOver` over a caller-supplied ground,
     `PaintAlpha` is the only real-alpha path (SCRIM / SURFACE_GLASS over the 3D world). Grounds
     are derived (`Ground`, `PanelGround`, `CardGround`, `SelectedCardGround`, `TintGround`).
     A one-shot `SelfCheck()` warns in the log if `Over()` ever drifts;
  3. fonts are layout-side (`FONT_HEAD / FONT_BODY / FONT_BODY_BOLD / FONT_MONO` name the GUIDs;
     `RADIUS_*` and the `TEXT_HEADER/BODY/MONO_SM/TAG` ladder sit next to them).
  Also the glass-chrome tokens of the Stitch pre-game mockups and the `TBD_EUITint` enum with
  `ChipFill/ChipBorder/ChipInk` and `PanelFill/PanelBorder` — the only place a chip or tinted
  panel colour is defined.
- `TBD_UIIcons.c`: mockup icon key → image. **Our textures first** (2026-09-14): 38 Material Symbols
  keys shipped as 64 px white-on-alpha PNGs under `UI/Textures/TBD/Icons/TBD_Icon_<key>_UI.png`
  (`Texture(key)`; bare `.edds` path until the operator imports the batch and pins the `.meta` GUIDs
  in `s_mTextureGuids`), then the measured vanilla `icons_wrapperUI-64.imageset` quads (`search`,
  `player`, `check`, `scenarios`, `cancel`, `settings`, `general`), else the slot hides and warns
  once. Regenerate / add a key from the repo root:
  `curl -s https://cdn.jsdelivr.net/npm/@material-symbols/svg-400/outlined/<key>.svg -o /tmp/<key>.svg && ffmpeg -i /tmp/<key>.svg -vf "scale=64:64:flags=lanczos,format=rgba,negate=negate_alpha=0" apps/mod/tbd-framework/UI/Textures/TBD/Icons/TBD_Icon_<key>_UI.png`
  and add the key to `BuildShipped()`.
- `TBD_UIScrollBar.c`: the 4 px scrollbar every TBD list wears (`Mount(dock, scroll, content,
  ground)`, `Destroy`). Not a widget handler: it ticks at 30 Hz through the call queue, sizes the
  thumb from viewport/content, follows `GetSliderPos`, hides when nothing scrolls. The engine's own
  bar is clipped away by the list layout (recipe in `UI/layouts/Common/README.md`).
- `TBD_MenuBase.c` & `TBD_MenuStack.c`: Chimera menu base class and stack manager governing
  window lifecycle, modal hierarchy, and Esc handling.
- `TBD_DockScreen.c`: base class of every Dock & Sub-Layout screen — `Mount(dock, layout)`,
  `MountHandler`, `UnmountAll`, the shared `TBD_SessionTopBar` / `TBD_SessionBottomBar` wiring,
  `GetOverlayDock()`, and the tab → preset routing (`PresetForTab`). Subclasses override
  `GetScreenTitle`, `GetSessionTab`, `GetSessionIdentity`, `OnBottomAction`.
- `TBD_ShellScreen.c`: Reusable full-screen workstation shell providing uniform header, list and
  one primary action (pre-dock screens: Spectator, Admin).
- `TBD_UIButton.c` & `TBD_UIInteractive.c`: Reusable interactive components normalizing mouse
  hover and gamepad focus behaviors. `TBD_UIButton` paints an optional `Border` image.
- `TBD_ListBox.c` & `TBD_ListBoxRow.c`: High-performance pooled list box system avoiding widget
  churn during scrolling; also the row engine inside `TBD_DropdownMenu`.

### Call Flow & Contracts
Pure client-side UI infrastructure. Consumed directly by all domain screens in `Session/*/UI/` and
shared widgets in `UI/Common/`. `TBD_DockScreen` subclasses call `super.OnScreenOpen()` first (the
bars are mounted there) and `super.OnScreenClose()` last (everything mounted is removed there).
