**Status:** live

# Full-screen workspaces documentation

The documentation of the web app's planned full-screen workspaces, the two that are not built and
so have no crate yet: the mission planner and the after-action review. Every built workspace is a
crate under `crates/frontend/workspaces/`, documented under
[workspace crate documentation](/documentation/crates/frontend/workspaces/README.md): the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) and the debug benches.
Developers and AI agents read it before starting a new workspace.

## Contents

```text
documentation/apps/frontend/workspaces/
├── aar/      the after-action review workspace, planned and not built: design notes and open work
└── planner/  the mission planner workspace, planned and not built: design notes and open work
```

## How it works

A planned workspace has no code, so its folder here mirrors no code folder: it holds a README
index and the feature doc of its design and open work. A workspace is a full-bleed, chromeless
route that mounts its own map canvas rather than a page inside the platform frame; the
[workspace crates README](/crates/frontend/workspaces/README.md) describes what the built ones
share.

| Workspace | Route | State | Start at |
|---|---|---|---|
| Mission Creator | `/missions/:id/edit`, and the mission hub's review workspace | built | [mission_creator_workspace/README.md](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) |
| Debug benches | `/debug/building-viewer`, `/debug/world-los`, `/debug/ballistics-agreement` | built | [debug_benches/README.md](/documentation/crates/frontend/workspaces/debug_benches/README.md) |
| Mission planner | none | planned; no code | [planner/README.md](/documentation/apps/frontend/workspaces/planner/README.md) |
| After-action review | none | planned; no code | [aar/README.md](/documentation/apps/frontend/workspaces/aar/README.md) |

A newly planned workspace gets a folder here with a README and its feature doc, a line in Contents
and a row in the table. When a planned workspace is built, it becomes a crate under
`crates/frontend/workspaces/` and its folder moves to
`documentation/crates/frontend/workspaces/<crate>/`, the crate's documentation mirror.

## Code

- [Frontend workspace crates](/crates/frontend/workspaces/README.md) — the built workspaces,
  each a crate, where a planned workspace is built.
- [Route table](/apps/frontend/src/app_routes.rs) and
  [route access table](/crates/frontend/foundation/frontend_route_table/src/routes.rs) — the workspace routes and their
  full-bleed, chromeless layout.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the workspace code and the ticket registry in
  `.ai/tickets/`, which the documents are written from.
- Used by: the [frontend documentation README](/documentation/apps/frontend/README.md),
  whose Contents names this folder; the
  [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md),
  which links it.
- Rules: one folder per planned workspace; a planned workspace's documents say it is not built and
  describe only its design and open work, never code that does not exist; a built workspace's
  documents live in its crate's documentation mirror, never here.

## Related documentation

- [Frontend documentation](/documentation/apps/frontend/README.md) — the route table from
  every route to its code folder and feature doc.
- [Page crate documentation](/documentation/crates/frontend/pages/README.md) — the routed pages
  inside the platform frame.
