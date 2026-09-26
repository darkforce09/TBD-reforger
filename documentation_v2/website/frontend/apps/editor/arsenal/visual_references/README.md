**Status:** live

# Arsenal design references

The design references of the [arsenal](/documentation_v2/glossary/a_to_f.md#arsenal), the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s loadout editor: two design-phase
sets of the "Loadout Forge" dialog, kept for colour and layout context rather than as an
implementation source.

## Contents

```text
documentation_v2/website/frontend/apps/editor/arsenal/visual_references/
├── arsenal_mockup/           the Loadout Forge dialog with four slot cards around the figure
└── loadout_forge_blueprint/  the Loadout Forge dialog with seven slot cards and its Stitch export
```

## How it works

A set is a folder named after its subject and kind. A blueprint holds the Stitch export as an html
file and its screenshot as a png, both named after the set; the mock-up holds only its screenshot.
Each set has a README that says what it shows and how the built tab differs. Both sets draw the
same dialog: a template list on the left, the slot cards of one
[slot](/documentation_v2/glossary/n_to_z.md#slot) in the middle, the weapon's configuration and the
"Standardization" buttons on the right, and "Cancel" and "Save & Close Forge" at the foot. Their
colour tokens are the Aegis export in the
[design system](/documentation_v2/design_system/token_exports/aegis_design_tokens.md). The built
UI is the Leptos code the Code section links; the
[Arsenal loadout editor](/documentation_v2/website/frontend/apps/editor/arsenal/arsenal_loadout_editor.md)
feature doc lists every difference in its Design section.

## Code

- [Arsenal](/apps/website/frontend/src/v2/apps/editor/arsenal/) — the built Arsenal tab the sets
  were drawn for.
- [Arsenal panels](/apps/website/frontend/src/v2/apps/editor/ui/arsenal/) — its doll,
  compatibility panel and cargo editor.

## Boundaries

- Depends on: nothing in the repository; the blueprint's html loads its styles and fonts
  from the network.
- Used by: the Arsenal loadout editor feature doc's Design section and the arsenal documentation
  README.
- Rules: a set is kept as captured and never edited to match the built tab; a new set gets its own
  folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a README;
  no screenshot of the built UI belongs here.
