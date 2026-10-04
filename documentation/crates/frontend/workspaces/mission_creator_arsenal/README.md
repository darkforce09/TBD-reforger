**Status:** live

# Arsenal documentation

The documentation of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[arsenal](/documentation/glossary/a_to_f.md#arsenal): the Attributes dialog's Arsenal tab, where a
mission maker edits one [slot](/documentation/glossary/n_to_z.md#slot)'s loadout, and the design
references drawn for it. Developers and AI agents read it before changing the loadout editor.

## Contents

```text
documentation/crates/frontend/workspaces/mission_creator_arsenal/
├── arsenal_loadout_editor.md  the Arsenal tab: flows, rules, data, design, open work and decisions
└── visual_references/         the design-phase mock-ups of the loadout editor
```

## How it works

[arsenal_loadout_editor.md](/documentation/crates/frontend/workspaces/mission_creator_arsenal/arsenal_loadout_editor.md)
is the feature doc and the place to start: what the tab shows, how a pick reaches the
[mission](/documentation/glossary/g_to_m.md#mission) document, what the rules refuse, and how the built
tab differs from the mock-ups. The code READMEs it links hold what the code declares: the folders,
the types and the tests that pin each rule. The feature inventory's
[attributes area](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/attributes_and_settings.md)
keeps the tab's inventory entry, ATTR-TAB-004, beside the dialog's other tabs.

`visual_references/` holds the two design-phase sets, a mock-up and a blueprint of the "Loadout
Forge" dialog; each set's README says what it shows and how the built tab differs from it.

## Code

- [Arsenal](/crates/frontend/workspaces/mission_creator_arsenal/src/) — the tab component, the loadout
  domain, its rules and the loadout commands.
- [Arsenal panels](/crates/frontend/workspaces/mission_creator_arsenal/src/panels.rs) — the doll host with its
  SVG paper doll, the compatibility panel and the cargo editor.
- [Doll preview](/crates/paper_doll/) — the paper doll crates' 3D doll the tab mounts.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md), the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md)
  and the [glossary](/documentation/glossary/README.md); the arsenal code, the registry handlers in
  `crates/api/api_missions/src/handlers/` and the ticket registry in `.ai/tickets/`.
- Used by: the [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md);
  the in-code READMEs of `crates/frontend/workspaces/mission_creator_arsenal/src/` and its folders,
  which link the feature doc under Related documentation.
- Rules: the feature doc describes the committed code and links the code READMEs rather than
  repeating their types; a set under `visual_references/` stays as captured.

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/attributes_and_settings.md)
  — the Attributes dialog the tab lives in.
- [Mission Creator roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md)
  — the Virtual Arsenal program and the open question of the mission armory.
