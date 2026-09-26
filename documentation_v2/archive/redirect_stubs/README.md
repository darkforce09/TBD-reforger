**Status:** live

# Redirect stubs

The stubs the retired docs/ tree kept at old document paths so that links to them still resolved,
each pointing at the document that took the path's place. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/redirect_stubs/
└── docs__*.md  one stub per old path, named after it: lowercased, each / as __, ending in .md
```

## How it works

A stub's name spells the path it stood at, lowercased: `docs__website__backend_architecture.md`
stood at docs/website/BACKEND_ARCHITECTURE.md, and the [Mission
Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) architecture stubs stood in
docs/specs/Mission_Creator_Architecture/ under their numbered names. Each stub is a few lines: a
"Moved" line that links its target and, where a live document now holds the subject, a status line
that links it. A link that still needs one of these documents goes to the target, never to the stub.

| Stubs | Target |
|---|---|
| the feature entry schema | [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md) |
| the two roadmaps | [Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md) |
| the two Eden UX specs | [Mission Creator UX spec](/documentation_v2/website/frontend/apps/editor/ux_spec.md) |
| the agent execution plan | [Mission Creator decisions](/documentation_v2/website/frontend/apps/editor/decisions.md) |
| the two feature inventories | [feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md) |
| the Eden editor feature reference | [Eden interactions](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md) |
| the Eden UI anatomy | [Eden UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md) |
| the Eden attribute catalog | [Eden attributes](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md) |
| the Eden gap analysis | [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md) |
| the Eden wiki manifest | [Eden wiki scrape manifest](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_wiki_scrape_manifest.yaml) |
| the problem statement, technical specification, engineering plan, macOS UX architecture, backend architecture, context handoffs and registration flow design | the matching files in [Go and React era designs](/documentation_v2/archive/go_and_react_era_design/README.md) |
| the Mission Creator mock-up readme | a GitHub permalink to the token folder it indexed |

## Code

None: the stubs point at documents, not code.

## Boundaries

- Depends on: the documents the stubs link, live and archived.
- Used by: one archived index in [monorepo merge
  records](/documentation_v2/archive/monorepo_migration/README.md) and the documentation program's
  own records; no live document links a stub.
- Rules: never reworded, only links change; a stub's target changes only when that document moves; a
  new link goes to the target, not the stub.

## Related documentation

- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — where
  most stubs point.
- [Go and React era designs](/documentation_v2/archive/go_and_react_era_design/README.md) — the
  archived targets of the rest.
