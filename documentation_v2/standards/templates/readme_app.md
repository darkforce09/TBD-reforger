**Status:** live

# README template: app

**When to use:** a workspace directly under `apps/website/frontend/src/v2/apps/`, a standalone tool
that mounts full screen from its own routes. The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
follows; the app kind adds Routes and Public surface.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there.

````markdown
# <Product name of the workspace: no path, no backticks>

<One to three sentences: what the workspace lets its users do, and what the folder holds.>

## Contents

```text
<repository path of the folder>/
├── <child folder>/  <what it is for: a lowercase phrase, no closing period>
├── mod.rs           <the module tree and the workspace's contract>
└── tests/           <what the tests cover>
```

## How it works

<How the workspace runs: what its route component mounts and boots, how input and panels reach the
engines, where its state lives, and the invariants every child keeps. Name each child's part in one
clause; the child's own README holds the detail.>

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| <path from app_routes.rs> | <component> | <tier from router.rs> | <full-bleed and chromeless flags> |

## Public surface

- <module::item>: <what it is, and who outside the workspace uses it>

## Boundaries

- Depends on: <the frontend core modules, engine crates and browser APIs the workspace uses>
- Used by: <the route table, and every page, core module or tool that reaches in, from git grep>
- Rules: <the invariants a change here must keep>

## Related documentation

- [<document title>](/documentation_v2/website/frontend/apps/<workspace>/<doc>.md) — <what it
  covers>
````

## Worked sample

Written from `apps/website/frontend/src/v2/apps/editor/`. The sample sits in a fenced block, so no
gate reads it as a README; the folder's own README.md is written from the same code and may differ.

````markdown
# Mission Creator

The Mission Creator: the 2D/3D CAD workspace in which mission makers build a mission on the map.
This folder holds the editor page, the chrome docked around the map, the canvas mount and its
overlays, the interactive map tools, the loadout editor and the browser session they all run in.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/
├── arsenal/           the loadout editor: loadout rows, compatibility rules, asset catalog, paper doll
├── bridge/            the engine seam: boot, document host, viewport, overlays, tactical graphics
├── input/             map pointer and keyboard events turned into map-engine commands; the map tools
├── mission_editor/    the page's parts: canvas mount, registry loading, placement, transform, toolbar
├── mission_editor.rs  `MissionEditorPage`, which mounts the canvas and raises the chrome around it
├── mod.rs             declares the six modules and states the workspace's contract
├── shell/             the browser session: drafts, hydrate, tab lock, review mode, preferences
├── tests/             the page's test suite, mounted from `mission_editor.rs`
└── ui/                docks, top strip, toolbelt, outliner, inspectors, Arsenal panels and dialogs
```

## How it works

`MissionEditorPage` mounts the canvas, boots the engine through `bridge/`, hydrates the mission
document from the server and the local draft (`shell/`), loads the item registry, and raises the
docks, toolbelt and overlays around the map. Pointer and keyboard input (`input/`) and every panel
under `ui/` change the document only through the map engine's hosted editing commands, which
`bridge/` hosts together with the document handle, the undo history and the selection; the panels
read signals and never write the document themselves. A module that touches `web_sys` or a live
engine handle compiles for `wasm32` only, so the native test build still compiles the pure half of
the workspace. When the review workspace opens an artifact's version, `shell/`'s review mode holds
it and every write path refuses.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | full-bleed, chromeless |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage`, from the mission hub's review workspace, which mounts `MissionEditorPage` in review mode | `mission_maker` | full-bleed, chromeless |

## Public surface

- `mission_editor::MissionEditorPage`: the route component, mounted by the route table and by the
  review workspace page.
- `shell::review_mode` (`ReviewedVersion`, `open`, `close`): the read-only review mode the review
  workspace opens before it mounts the editor.
- `shell::hydrate::purge_local_documents`: drops an account's local drafts; the auth store calls it
  when the account signs out.
- `shell::mission_size::format_bytes`: the payload-size formatter the mission library's upload
  panel reuses.
- `shell::layout::{HOVER_FILL, DISABLED_GLYPH}`: the chrome's hover and disabled classes, reused by
  the core search box, select and slider.

## Boundaries

- Depends on: `crate::v2::core` (the API client and DTOs, the auth store, the UI primitives, the test
  support), `website_map_engine` (its `data`, `editing`, `streaming`, `overlay`, `frame`, `camera`,
  `spatial`, `world` and `doll` modules) and `web_sys` in the browser build.
- Used by:
  - `apps/website/frontend/src/app_routes.rs`, the route table;
  - `apps/website/frontend/src/v2/pages/mission_hub/review_workspace/page.rs`, for the page and the
    review mode;
  - `apps/website/frontend/src/v2/pages/mission_hub/library/dossier_upload.rs` and
    `dossier_upload_panel.rs`, for `format_bytes`;
  - `apps/website/frontend/src/v2/core/auth/store.rs`, for `purge_local_documents`;
  - the core search box, select and slider in `apps/website/frontend/src/v2/core/ui/`, for the
    layout classes;
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`, which drive the
    `/missions/:id/edit` route.
- Rules: a document mutation goes through `website_map_engine::editing`, never straight out of a
  panel; a module that touches `web_sys` or a live engine handle is `#[cfg(target_arch = "wasm32")]`,
  and so is its `pub mod` line; no sibling workspace reaches in.

## Related documentation

- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  feature inventory, the UX specification, the decisions and the roadmap.
- [Editor gates runbook](/documentation_v2/runbooks/editor_gates.md) — running the headless editor
  gates.
````
