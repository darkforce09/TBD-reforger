**Status:** live

# Mission planner documentation

The documentation of the mission planner, a planned workspace that is not built: a tactical
whiteboard on which squad leaders and commanders draw their plan over a published
[mission](/documentation_v2/glossary/g_to_m.md#mission) before an
[event](/documentation_v2/glossary/a_to_f.md#event), without editing the mission. Developers and AI agents
read it before starting the workspace.

## Contents

```text
documentation_v2/website/frontend/apps/planner/
└── mission_planner.md  the planner's design notes, what exists to build on, and its open work
```

## Code

- [Mission planner workspace](/apps/website/frontend/src/v2/apps/planner/) — the reserved code
  folder, which holds only its README: no module, no route.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md); the
  design notes of the product blueprint, archived in
  `documentation_v2/archive/go_and_react_era_design/mission_creator_design.md`; the ticket registry
  in `.ai/tickets/`.
- Used by: the [full-screen workspaces documentation](/documentation_v2/website/frontend/apps/README.md)
  and the in-code README of the reserved folder.
- Rules: the documents describe a planned workspace and say so; they never describe code that does
  not exist as if it did, and they move to the built behaviour once the workspace lands.
