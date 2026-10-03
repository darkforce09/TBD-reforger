**Status:** live

# Mission planner documentation

The documentation of the mission planner, a planned workspace that is not built: a tactical
whiteboard on which squad leaders and commanders draw their plan over a published
[mission](/documentation/glossary/g_to_m.md#mission) before an
[event](/documentation/glossary/a_to_f.md#event), without editing the mission. Developers and AI agents
read it before starting the workspace.

## Contents

```text
documentation/apps/frontend/workspaces/planner/
└── mission_planner.md  the planner's design notes, what exists to build on, and its open work
```

## Code

- [Mission planner workspace](/apps/frontend/src/workspaces/planner/) — the reserved code
  folder, which holds only its README: no module, no route.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  design notes of the product blueprint, archived in
  `documentation/archive/go_and_react_era_design/mission_creator_design.md`; the ticket registry
  in `.ai/tickets/`.
- Used by: the [full-screen workspaces documentation](/documentation/apps/frontend/workspaces/README.md)
  and the in-code README of the reserved folder.
- Rules: the documents describe a planned workspace and say so; they never describe code that does
  not exist as if it did, and they move to the built behaviour once the workspace lands.
