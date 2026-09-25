**Status:** live

# Slot kit inspector mockup

Design-phase reference for the lobby's kit inspector: the chosen seat's loadout, card by card. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/lobby/visual_references/slot_kit_inspector_mockup/
├── slot_kit_inspector_mockup.html  the Stitch export
└── slot_kit_inspector_mockup.png   its screenshot
```

## How it works

The set shows "KIT INSPECTOR" with the seat line "8: RIFLEMAN (AT)", weapon chips (`AK-74`, `RPG-7`), the squad "Alpha 2-1", a Gear card (helmet, vest, jacket, pants, boots, gloves, backpack) and weapon slot cards with their mounted attachments and ammunition.

The built column, `TBD_KitInspectorPanel` in the `TBD_KitInspector` layout, follows the set: a 3D preview of the kit on a character (`TBD_KitPreviewComponent`), then cards for gear, weapons, grenades, gadgets, tools, medical and miscellaneous items; it reads the mock catalog's kits. The [lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) feature doc holds the full comparison.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
