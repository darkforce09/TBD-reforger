**Status:** live

# Arsenal mock-up

Design-phase reference for the [arsenal](/documentation_v2/glossary.md#arsenal), the Arsenal tab of
the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s Attributes dialog: a
"Loadout Forge" dialog that arms one [slot](/documentation_v2/glossary.md#slot). It gives colour
and layout context and is not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/apps/editor/arsenal/`.

## Contents

```text
documentation_v2/website/frontend/apps/editor/arsenal/visual_references/arsenal_mockup/
└── arsenal_mockup.png  the screenshot of the mock-up; it has no html export
```

## How it works

The mock-up shows the breadcrumb "US Army › Alpha Squad › Rifleman", a "Search templates..."
field over the tabs Factions, Uniforms, Weapons and a cut-off fourth, two template cards ("Standard M4A1 Kit",
"Heavy Gunner M249"), the heading "ALPHA 1-1 RIFLEMAN" over a figure with Helmet, Uniform and Vest
cards and a "Primary Weapon" card for the "M4A1 Carbine", an "M4A1 Configuration" panel with
Optic and Muzzle dropdowns and an "Ammunition" row ("5.56x45mm NATO", "x7 Mags"), a
"Standardization" block with "Apply Kit to Entire Squad" and "Apply Kit to Entire Faction", and
the buttons "Cancel" and "Save & Close Forge".

The built tab differs: it is a tab of the Attributes dialog rather than its own dialog; it has no
templates, breadcrumb or Save and Cancel buttons, since every pick is written to the
[mission](/documentation_v2/glossary.md#mission) at once; a rail of 14 regions, a filtered item
list and a 3D doll replace the three cards; attachments are compatibility toggles, and ammunition
is cargo in a worn container; and Copy and Apply over the current selection stand in for the two
"Apply Kit" buttons. The
[Arsenal loadout editor](/documentation_v2/website/frontend/apps/editor/arsenal/arsenal_loadout_editor.md)
feature doc holds the full comparison.

## Code

- [Arsenal](/apps/website/frontend/src/v2/apps/editor/arsenal/) — the tab this set was drawn for.

## Boundaries

- Depends on: nothing; the png is self-contained.
- Used by: the Arsenal loadout editor feature doc's Design section and the arsenal visual
  references README.
- Rules: the set is kept as captured.
