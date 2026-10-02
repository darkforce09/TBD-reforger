# Theme, layouts and icons

The look of every TBD screen in one place: the colour tokens and the sRGB paint helpers, the tint
and state colour tables with their two vocabularies, the registry of every layout and texture
resource, and the icon lookup.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/
├── TBD_EUIState.c        TBD_EUIState: the semantic state of an interactive surface
├── TBD_EUITint.c         TBD_EUITint: the tint vocabulary of chips, badges and tinted panels
├── TBD_UIIcons.c         TBD_UIIcons: icon key to the addon's texture or a vanilla imageset quad
├── TBD_UILayouts.c       TBD_UILayouts: every layout and texture resource, and the create helpers
├── TBD_UIStateColours.c  TBD_UIStateColours: row background, title, detail and accent per state
├── TBD_UITheme.c         TBD_UITheme: colour tokens, sRGB compositing and paint, type and radius
└── TBD_UITintColours.c   TBD_UITintColours: chip, panel and faction-row colours per tint
```

## How it works

A screen names a colour token from `TBD_UITheme` and paints with `Paint` (over the glass panel
ground), `PaintOver` (over a ground it knows) or `PaintAlpha` (real engine alpha, only over the 3D
world). Tokens are sRGB; `Over(top, ground)` composites a translucent token in sRGB and `Colour`
converts through `Color.FromSRGBA`, memoised per token. A component that shows a tint or a state
asks `TBD_UITintColours` or `TBD_UIStateColours` for the token and paints it the same way.

`TBD_UILayouts` names every layout and texture as `"{GUID}path"` and creates layouts through
`Create` (bare-path retry), `CreateStretched`, `CreateHandler` and `MountRounded`, which fills a
`*Border` or `*BG` frame dock with the rounded shape of the nearest radius. `TBD_UIIcons.Load` shows
the addon's own icon raster for its 38 shipped keys, else a vanilla imageset quad that resolved on a
real run, else hides the slot and warns once through `TBD_WarnOnce`.

## Authority

- Server: nothing; without a workspace `TBD_UILayouts.Create` returns null.
- Client: everything; the classes run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's widget API; the layouts under `apps/mod/tbd-framework/UI/layouts/` and
  the textures under `apps/mod/tbd-framework/UI/Textures/`; `TBD_WarnOnce` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/Logging/`.
- Used by: every screen, component and HUD painter under
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/` and
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the mock catalogs, which name hero textures.
- Rules: a colour is a `TBD_UITheme` token, a `TBD_EUITint` or a `TBD_EUIState`, never a literal;
  a resource is named once, in `TBD_UILayouts`, and a new GUID block is checked against the ledger
  in its class banner; the `TEXT_*` ladder is copied by hand into the `.layout` files; lines added
  to a script stay ASCII, and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Design tokens](/documentation/design_system/design_tokens.md) — the token vocabulary shared
  with the website, and the known differences
