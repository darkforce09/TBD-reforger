**Status:** live

# Full-screen workspaces documentation

The documentation of the web app's full-screen workspaces, one folder per workspace folder of the
code: the [Mission Creator](/documentation_v2/glossary.md#mission-creator), the debug benches, and
the two workspaces that are planned but not built, the mission planner and the after-action
review. Developers and AI agents read it before changing a workspace or starting a new one.

## Contents

```text
documentation_v2/website/frontend/apps/
├── aar/      the after-action review workspace, planned and not built: design notes and open work
├── debug/    the debug benches: the building viewer and the world line-of-sight bench
├── editor/   the Mission Creator: feature inventory, UX, roadmap, decisions, Eden reference, arsenal
└── planner/  the mission planner workspace, planned and not built: design notes and open work
```

## How it works

The folders mirror the workspace folders under `apps/website/frontend/src/v2/apps/` and keep their
spelling. Each holds a README index and its feature docs; the Mission Creator's folder also holds
its reference catalogs and design references, and splits its feature docs into subfolders. A
workspace is a full-bleed, chromeless route that mounts its own map canvas rather than a page
inside the platform frame; the
[workspaces README](/apps/website/frontend/src/v2/apps/README.md) describes what they share.

| Workspace | Route | State | Start at |
|---|---|---|---|
| Mission Creator | `/missions/:id/edit`, and the mission hub's review workspace | built | [editor/README.md](/documentation_v2/website/frontend/apps/editor/README.md) |
| Debug benches | `/debug/building-viewer`, `/debug/world-los` | built | [debug/README.md](/documentation_v2/website/frontend/apps/debug/README.md) |
| Mission planner | none | planned; the code folder holds only its README | [planner/README.md](/documentation_v2/website/frontend/apps/planner/README.md) |
| After-action review | none | planned; the code folder holds only its README | [aar/README.md](/documentation_v2/website/frontend/apps/aar/README.md) |

A new workspace gets a folder here named like its code folder, with a README and its feature docs,
and a line in Contents and a row in the table.

## Code

- [Full-screen workspaces](/apps/website/frontend/src/v2/apps/) — the workspace folders these
  documents describe, and the module tree that declares the built ones.
- [Route table](/apps/website/frontend/src/app_routes.rs) and
  [route access table](/apps/website/frontend/src/router.rs) — the workspace routes and their
  full-bleed, chromeless layout.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation_v2/glossary.md); the workspace code and the ticket registry in
  `.ai/tickets/`, which the documents are written from.
- Used by: the [frontend documentation README](/documentation_v2/website/frontend/README.md),
  whose Contents names this folder; the in-code READMEs of the workspace folders, which link their
  documents under Related documentation.
- Rules: one folder per workspace folder of the code, spelled the same; a planned workspace's
  documents say it is not built and describe only its design and open work, never code that does
  not exist.

## Related documentation

- [Frontend documentation](/documentation_v2/website/frontend/README.md) — the route table from
  every route to its code folder and feature doc.
- [Pages documentation](/documentation_v2/website/frontend/pages/README.md) — the routed pages
  inside the platform frame.
