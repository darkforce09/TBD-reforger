**Status:** live

# Voice panel mockup

Design-phase reference for a voice panel for the pre-game screens: who is on which voice channel. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/voice_panel_mockup/
├── voice_panel_mockup.html  the Stitch export
└── voice_panel_mockup.png   its screenshot
```

## How it works

The set shows "VOICE" "CONNECTED" with the in-game net, an auto-move toggle, "BRIEFING CHANNELS" per side (BLUFOR 18, OPFOR 12) with their members, and "OTHER CHANNELS".

No voice panel is built: the lobby's faction column keeps an empty `VoiceDock` for it, and no script mounts anything there. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the empty `VoiceDock` kept for this panel.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
