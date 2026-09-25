**Status:** live

# After-action review documentation

The documentation of the after-action review, a planned workspace that is not built: a replay of
a finished match from server telemetry over a read-only map, for members and leaders reviewing an
[event](/documentation_v2/glossary.md#event) after it ends. Developers and AI agents read it
before starting the workspace.

## Contents

```text
documentation_v2/website/frontend/apps/aar/
└── after_action_review.md  the replay's design notes, the telemetry it needs, and its open work
```

## Code

- [After-action review workspace](/apps/website/frontend/src/v2/apps/aar/) — the reserved code
  folder, which holds only its README: no module, no route.
- [Match telemetry domain](/apps/website/api_v2/src/match_telemetry/) — the match results the
  game servers report today, including the optional replay link.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md); the
  design notes of the product blueprint, archived in
  `documentation_v2/archive/go_and_react_era_design/mission_creator_design.md`; the ticket registry
  in `.ai/tickets/`.
- Used by: the [full-screen workspaces documentation](/documentation_v2/website/frontend/apps/README.md)
  and the in-code README of the reserved folder.
- Rules: the documents describe a planned workspace and say so; they never describe code that does
  not exist as if it did, and they move to the built behaviour once the workspace lands.
