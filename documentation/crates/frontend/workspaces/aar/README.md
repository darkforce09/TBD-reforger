**Status:** live

# After-action review documentation

The documentation of the after-action review, a planned workspace that is not built: a replay of
a finished match from server telemetry over a read-only map, for members and leaders reviewing an
[event](/documentation/glossary/a_to_f.md#event) after it ends. Developers and AI agents read it
before starting the workspace.

## Contents

```text
documentation/crates/frontend/workspaces/aar/
└── after_action_review.md  the replay's design notes, the telemetry it needs, and its open work
```

## Code

- None yet: no crate, module or route exists for the workspace. It is built as one crate under
  [frontend workspace crates](/crates/frontend/workspaces/README.md); until then
  `crates/frontend/shell/frontend_application/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs` have no replay route, and
  nothing compiles for it.
- [Match telemetry domain](/crates/api/api_match_telemetry/src/) — the match results the
  game servers report today, including the optional replay link.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  design notes of the product blueprint, archived in
  `documentation/archive/go_and_react_era_design/mission_creator_design.md`; the ticket registry
  in `.ai/tickets/`.
- Used by: the [workspace crate documentation](/documentation/crates/frontend/workspaces/README.md),
  the [glossary](/documentation/glossary/README.md) and the
  [product roadmap](/documentation/product_roadmap.md), which link the feature doc.
- Rules: the documents describe a planned workspace and say so; they never describe code that does
  not exist as if it did, and they move to the built behaviour once the workspace lands. The
  workspace is added as a crate under `crates/frontend/workspaces/` with its manifest, README,
  route component and a row in both `crates/frontend/shell/frontend_application/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`; it depends on the foundation
  and feature crates and the map crates, never on a page crate or another workspace's crates; its
  documents then move to `documentation/crates/frontend/workspaces/<crate>/`.
