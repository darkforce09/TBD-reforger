**Status:** live

# Identity link modal sheet mockup

Design-phase reference for the identity link: a modal sheet in which a player links the game to
their platform account. It gives layout and colour context and is not an implementation source;
the built flow is the chat command the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/discord_identity_link/visual_references/identity_link_macos_modal_sheet_mockup/
├── identity_link_macos_modal_sheet_mockup.html  the Stitch export
└── identity_link_macos_modal_sheet_mockup.png   its screenshot
```

## How it works

The set shows a status bar ("● UNLINKED — No web profile associated with this Bohemia UID" and a
platform UID), the heading "Connect Bohemia Reforger to TBD Operations" with three benefits
([ORBAT](/documentation_v2/glossary.md#orbat) [slot](/documentation_v2/glossary.md#slot) reservation, combat telemetry, Discord rank sync), a one-time passcode card (`TBD-8X2K`,
"Expires in: 05:04", "COPY CODE" and a regenerate button), "METHOD A" (a browser link page with
three steps) and "METHOD B" (a Discord bot's `/link` command), a "Do not show again on connect"
check and "CLOSE".

The built flow has no dialog: the player generates a 6-digit code on the website and types
`#tbd link <code>` in game, and the server answers in private chat. The
[identity link specification](/documentation_v2/mod/tbd-framework/UI/discord_identity_link/discord_identity_link_specification.md)
feature doc holds the full comparison.

## Code

- [Backend bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/) — the built command this set
  was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
