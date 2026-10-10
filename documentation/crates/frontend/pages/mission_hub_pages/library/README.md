**Status:** live

# Mission library page documentation

The feature documentation of the `/missions` page, where members browse
[missions](/documentation/glossary/g_to_m.md#mission), open a mission's dossier in a slide-over, create
a new mission and manage their own, with the page's design-phase reference.

## Contents

```text
documentation/crates/frontend/pages/mission_hub_pages/library/
├── mission_library_page.md  the feature doc: browsing, the dossier, creation, upload and management
└── visual_references/       the design-phase blueprint of a hero above a filterable card grid
```

## How it works

Read [mission_library_page.md](/documentation/crates/frontend/pages/mission_hub_pages/library/mission_library_page.md)
first. It follows the [feature doc template](/documentation/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what each call means in the
[API](/documentation/glossary/a_to_f.md#api), including the controls the API refuses to a demoted
author, and covers the New Mission dialog, which has no route of its own. The blueprint in
`visual_references/` is a design-phase reference: it has no New Mission button, labels its
filters and offers a custom map, while the built page adds bookmarks and status badges. The code
folders' READMEs list the files.

## Code

- [Mission library page](/crates/frontend/pages/mission_hub_pages/src/library/) — the route
  component `MissionLibraryPage`, the grid, the hero and the dossier sheet.
- [New mission dialog](/crates/frontend/pages/mission_hub_pages/src/create_dialog/) — the
  create form the library opens.
- [Missions domain](/crates/api/api_missions/src/) — the list, detail, bookmark, lifecycle,
  version and submission routes the page calls.

## Boundaries

- Depends on: the feature doc template; the page and dialog code, the missions handlers and the
  ticket registry the feature doc is written from.
- Used by: the in-code READMEs of the library, the create dialog and the mission hub, and the
  missions domain README, which link the feature doc; the mission hub pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Missions domain](/crates/api/api_missions/src/README.md) — the API side of the library,
  the lifecycle and the versions.
