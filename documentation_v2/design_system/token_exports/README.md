**Status:** live

# Design token exports

The design token sheets that Stitch exported with the mockups: design-phase references kept for
colour and layout context, not implementation sources. The built interface is the Leptos code under
`apps/website/frontend/src/v2/` and the mod's `TBD_UITheme`; the
[design tokens](/documentation_v2/design_system/design_tokens.md) document says what the code
uses.

## Contents

```text
documentation_v2/design_system/token_exports/
├── aegis_design_tokens.md                     Aegis Tactical Command: the website and Mission Creator mockups
├── dark_tactical_operations_design_tokens.md  Dark Tactical Operations: the mod's in-game HUD and spectator mockups
└── reforger_dark_tactical_design_tokens.md    Reforger Dark Tactical: the mod's end screen and debrief mockups
```

## How it works

Each file is a Stitch export kept byte for byte: YAML front matter with the `colors`,
`typography`, `rounded` and `spacing` tokens, then Stitch's prose on brand, colour, type, layout,
elevation, shapes and components. Because they are generated, they carry no status line, the
`.editorconfig-checker.json` exclusions skip them, and the ticket engine's stale-identifier scan
exempts them (`SCAN_EXEMPT_PREFIXES` in `tools_v2/ticket-engine/src/repository.rs`); the Aegis
file also ends without a final newline. `dark_tactical_operations_design_tokens.md` holds a second
export pasted after its own prose, a Stitch screen path and the Reforger Dark Tactical front
matter, kept as exported.

| Export | Primary | Surface | Text | Design target of |
|---|---|---|---|---|
| Aegis Tactical Command | `#d8e2ff`, container `#adc6ff`; `#3b82f6` as the action blue | `#0d1322` | `#dde2f7` | the website and the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator), the arsenal included |
| Dark Tactical Operations | `#adc6ff`, container `#4d8eff` | `#0d1322` | `#dde2f8` | the mod's HUD and spectator screens |
| Reforger Dark Tactical | `#3b82f6` | `#0b1120` | `#ffffff` | the mod's end screen and debrief, with BLUFOR `#3b82f6`, OPFOR `#ef4444` and INDFOR `#22c55e` |

The built palette takes Aegis's surfaces and text and makes `#adc6ff` the primary with `#3b82f6`
as `action`; the design tokens document lists every difference. A new export lands here only when
several screens share it; one screen's tokens go into its set's `design_tokens.md` under
`visual_references/`.

## Code

- [Aegis stylesheet](/apps/website/frontend/style/README.md) — the website's built tokens, which
  follow the Aegis export.
- [Mod interface core](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/README.md) — `TBD_UITheme`,
  which mirrors the website's tokens and carries the mod mockups' extra colours.

## Boundaries

- Depends on: Stitch, which produced the exports.
- Used by: the Mission Creator's and the arsenal's `visual_references/` READMEs and the Mission
  Creator prototype set, which link the Aegis export; the design tokens document;
  `.editorconfig-checker.json` and `tools_v2/ticket-engine/src/repository.rs`, which exempt this
  folder by path.
- Rules: an export is never edited, only replaced by a new export; its differences from the built
  interface are written in the design tokens document or the screen's feature doc, never into the
  export.

## Related documentation

- [Mission Creator visual references](/documentation_v2/website/frontend/apps/editor/visual_references/README.md)
  — the mockups the Aegis export belongs to.
- [Spectator visual references](/documentation_v2/mod/tbd-framework/UI/spectator/visual_references/README.md),
  [end screen visual references](/documentation_v2/mod/tbd-framework/UI/end_screen/visual_references/README.md)
  and [debrief visual references](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/visual_references/README.md)
  — the mod mockups the other two exports come from.
