**Status:** live

# Workspace crate documentation

The feature documentation of the workspace crates under `crates/frontend/workspaces/`: one folder
per crate, and in it the feature docs of that workspace's benches or tools. Developers and AI
agents read it before changing a workspace.

## Contents

```text
documentation/crates/frontend/workspaces/
├── debug_benches/              the URL-only debug benches: building viewer, world line of sight, ballistics agreement
├── mission_creator_arsenal/    the Mission Creator's Arsenal: the loadout editor tab, its flows, rules and design references
└── mission_creator_workspace/  the Mission Creator: feature inventory, UX, roadmap, decisions, Eden reference
```

## Code

- [Frontend workspace crates](/crates/frontend/workspaces/README.md) — the crates the documents
  below describe.

## Boundaries

- Depends on: the code of `crates/frontend/workspaces/`; the
  [feature doc template](/documentation/standards/templates/feature_doc.md) and the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md).
- Used by: the [frontend crate documentation](/documentation/crates/frontend/README.md) index; the
  workspace crates' READMEs, which link their feature docs.
- Rules: one folder per workspace crate, spelled like the crate; below it the folders mirror the
  crate's `src/` folders.

## Related documentation

- [Frontend documentation](/documentation/apps/frontend/README.md) — every route with its code
  folder and feature doc.
