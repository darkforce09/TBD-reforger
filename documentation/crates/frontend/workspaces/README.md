**Status:** live

# Workspace crate documentation

The feature documentation of the web app's full-screen workspaces: one folder per workspace crate
under `crates/frontend/workspaces/`, and in it the feature docs of that workspace's benches or
tools, and one folder per planned workspace that is not built and so has no crate yet, the mission
planner and the after-action review. Developers and AI agents read it before changing a workspace
or starting a new one.

## Contents

```text
documentation/crates/frontend/workspaces/
├── aar/                        the after-action review workspace, planned and not built: design notes and open work
├── debug_benches/              the URL-only debug benches: building viewer, world line of sight, ballistics agreement
├── mission_creator_arsenal/    the Mission Creator's Arsenal: the loadout editor tab, its flows, rules and design references
├── mission_creator_workspace/  the Mission Creator: feature inventory, UX, roadmap, decisions, Eden reference
└── planner/                    the mission planner workspace, planned and not built: design notes and open work
```

## How it works

A workspace is a full-bleed, chromeless route that mounts its own map canvas rather than a page
inside the platform frame; the [workspace crates README](/crates/frontend/workspaces/README.md)
describes what the built ones share. A built workspace's folder is its crate's documentation
mirror: it is spelled like the crate, and below it the folders mirror the crate's `src/` folders.
A planned workspace has no code, so its folder mirrors no code folder: it holds a README index and
the feature doc of its design and open work.

| Workspace | Route | State | Start at |
|---|---|---|---|
| Mission Creator | `/missions/:id/edit`, and the mission hub's review workspace | built | [mission_creator_workspace/README.md](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) |
| Debug benches | `/debug/building-viewer`, `/debug/world-los`, `/debug/ballistics-agreement` | built | [debug_benches/README.md](/documentation/crates/frontend/workspaces/debug_benches/README.md) |
| Mission planner | none | planned; no code | [planner/README.md](/documentation/crates/frontend/workspaces/planner/README.md) |
| After-action review | none | planned; no code | [aar/README.md](/documentation/crates/frontend/workspaces/aar/README.md) |

A newly planned workspace gets a folder here with a README and its feature doc, a line in Contents
and a row in the table. When a planned workspace is built, it becomes a crate under
`crates/frontend/workspaces/` and its folder takes the crate's name, becoming the crate's
documentation mirror.

## Code

- [Frontend workspace crates](/crates/frontend/workspaces/README.md) — the crates the documents
  below describe, where a planned workspace is built.
- [Route table](/crates/frontend/shell/frontend_application/src/app_routes.rs) and
  [route access table](/crates/frontend/foundation/frontend_route_table/src/routes.rs) — the
  workspace routes and their full-bleed, chromeless layout.

## Boundaries

- Depends on: the code of `crates/frontend/workspaces/`; the
  [feature doc template](/documentation/standards/templates/feature_doc.md) and the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the ticket registry in `.ai/tickets/`, which
  the planned workspaces' documents are written from.
- Used by: the [frontend crate documentation](/documentation/crates/frontend/README.md) index; the
  workspace crates' READMEs, which link their feature docs; the
  [frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md)
  hub.
- Rules: one folder per workspace crate, spelled like the crate, and below it the folders mirror the
  crate's `src/` folders; one folder per planned workspace, whose documents say it is not built and
  describe only its design and open work, never code that does not exist; a built workspace's
  documents live in its crate's folder.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) —
  every route with its code folder and feature doc.
- [Page crate documentation](/documentation/crates/frontend/pages/README.md) — the routed pages
  inside the platform frame.
