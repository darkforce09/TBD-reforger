**Status:** live

# Frequencies panel mockup

Design-phase reference for the briefing's Frequencies page: the side's radio nets. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/frequencies_panel_mockup/
├── frequencies_panel_mockup.html  the Stitch export
└── frequencies_panel_mockup.png   its screenshot
```

## How it works

The set shows "Radio Frequencies Net", the "Long Range (LR) Command" net at 76.2 MHz with "Auxiliary Fallback:" channels, and "Short Range (SR) Squad Nets" such as "A1-1 Company HQ (Mission Maker)" at 390.7 MHz with "Aux Channels:".

The built page, `TBD_BriefingFrequenciesPage` with one `TBD_FreqRow` per net, follows the set and adds an `N nets` count; its nets come from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
