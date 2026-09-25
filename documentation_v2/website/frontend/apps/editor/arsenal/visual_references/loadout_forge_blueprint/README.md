**Status:** live

# Loadout Forge blueprint

Design-phase reference for the [arsenal](/documentation_v2/glossary.md#arsenal), the Arsenal tab
of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s Attributes dialog: the
"Loadout Forge" dialog, titled "Loadout Forge - Aegis Tactical" in its export, that arms one
[slot](/documentation_v2/glossary.md#slot). It gives colour and layout context and is not an
implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/apps/editor/arsenal/`.

## Contents

```text
documentation_v2/website/frontend/apps/editor/arsenal/visual_references/loadout_forge_blueprint/
├── loadout_forge_blueprint.html  the Stitch export of the Loadout Forge dialog
└── loadout_forge_blueprint.png   its screenshot
```

## How it works

The blueprint draws the same dialog as the
[Arsenal mock-up](/documentation_v2/website/frontend/apps/editor/arsenal/visual_references/arsenal_mockup/README.md)
with a fuller figure: Helmet, Vest and Backpack cards on the left of the "M4A1 Carbine" card and
Uniform, Secondary ("PISTOL") and Medical cards on its right. The export names the cut-off
fourth template tab Medical and holds each dropdown's choices: Optic "ACOG 4x (Standard)", "Holo
Sight", "Red Dot" and "Iron Sights"; Muzzle "Suppressor (Tactical)", "Compensator", "Flash Hider"
and "Standard Barrel". Breadcrumb, template cards, Ammunition row, "Standardization" block and the
"Cancel" and "Save & Close Forge" buttons are as in the mock-up.

The built tab differs as it does from the mock-up: a tab rather than a dialog; no templates,
breadcrumb, Save or Cancel, since every pick is written to the
[mission](/documentation_v2/glossary.md#mission) at once; a rail of 14 regions, a filtered item
list and a 3D doll instead of seven cards; attachments offered by the compatibility graph rather
than fixed dropdowns; ammunition as cargo; and Copy and Apply over the selection instead of
"Apply Kit to Entire Squad" and "Apply Kit to Entire Faction". The
[Arsenal loadout editor](/documentation_v2/website/frontend/apps/editor/arsenal/arsenal_loadout_editor.md)
feature doc holds the full comparison.

## Code

- [Arsenal](/apps/website/frontend/src/v2/apps/editor/arsenal/) — the tab this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts (the text faces and the Material Symbols
  icons), which the html loads when opened; the png needs nothing.
- Used by: the Arsenal loadout editor feature doc's Design section and the arsenal visual
  references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
