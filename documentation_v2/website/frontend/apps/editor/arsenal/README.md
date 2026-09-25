**Status:** live

# Arsenal documentation

The documentation of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
[arsenal](/documentation_v2/glossary.md#arsenal): the Attributes dialog's Arsenal tab, where a
mission maker edits one [slot](/documentation_v2/glossary.md#slot)'s loadout, and the design
references drawn for it. Developers and AI agents read it before changing the loadout editor.

## Contents

```text
documentation_v2/website/frontend/apps/editor/arsenal/
├── arsenal_loadout_editor.md  the Arsenal tab: flows, rules, data, design, open work and decisions
└── visual_references/         the design-phase mock-ups of the loadout editor
```

## How it works

[arsenal_loadout_editor.md](/documentation_v2/website/frontend/apps/editor/arsenal/arsenal_loadout_editor.md)
is the feature doc and the place to start: what the tab shows, how a pick reaches the
[mission](/documentation_v2/glossary.md#mission) document, what the rules refuse, and how the built
tab differs from the mock-ups. The code READMEs it links hold what the code declares: the folders,
the types and the tests that pin each rule. The feature inventory's
[attributes area](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md)
keeps the tab's inventory entry, ATTR-TAB-004, beside the dialog's other tabs.

`visual_references/` holds the two design-phase sets, a mock-up and a blueprint of the "Loadout
Forge" dialog; each set's README says what it shows and how the built tab differs from it.

## Code

- [Arsenal](/apps/website/frontend/src/v2/apps/editor/arsenal/) — the tab component, the loadout
  domain, its rules and the loadout commands.
- [Arsenal panels](/apps/website/frontend/src/v2/apps/editor/ui/arsenal/) — the doll host with its
  SVG paper doll, the compatibility panel and the cargo editor.
- [Doll preview](/apps/website/map-engine/src/doll/) — the map engine's 3D doll the tab mounts.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md), the
  [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md)
  and the [glossary](/documentation_v2/glossary.md); the arsenal code, the registry handlers in
  `apps/website/api_v2/src/missions/handlers/` and the ticket registry in `.ai/tickets/`.
- Used by: the [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md);
  the in-code READMEs of `apps/website/frontend/src/v2/apps/editor/arsenal/` and its folders,
  which link the feature doc under Related documentation.
- Rules: the feature doc describes the committed code and links the code READMEs rather than
  repeating their types; a set under `visual_references/` stays as captured.

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md)
  — the Attributes dialog the tab lives in.
- [Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)
  — the Virtual Arsenal program and the open question of the mission armory.
