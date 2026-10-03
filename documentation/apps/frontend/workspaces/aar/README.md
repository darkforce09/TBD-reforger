**Status:** live

# After-action review documentation

The documentation of the after-action review, a planned workspace that is not built: a replay of
a finished match from server telemetry over a read-only map, for members and leaders reviewing an
[event](/documentation/glossary/a_to_f.md#event) after it ends. Developers and AI agents read it
before starting the workspace.

## Contents

```text
documentation/apps/frontend/workspaces/aar/
└── after_action_review.md  the replay's design notes, the telemetry it needs, and its open work
```

## Code

- [After-action review workspace](/apps/frontend/src/workspaces/aar/) — the reserved code
  folder, which holds only its README: no module, no route.
- [Match telemetry domain](/crates/api/api_match_telemetry/src/) — the match results the
  game servers report today, including the optional replay link.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  design notes of the product blueprint, archived in
  `documentation/archive/go_and_react_era_design/mission_creator_design.md`; the ticket registry
  in `.ai/tickets/`.
- Used by: the [full-screen workspaces documentation](/documentation/apps/frontend/workspaces/README.md)
  and the in-code README of the reserved folder.
- Rules: the documents describe a planned workspace and say so; they never describe code that does
  not exist as if it did, and they move to the built behaviour once the workspace lands.
