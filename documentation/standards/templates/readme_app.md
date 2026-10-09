**Status:** live

# README template: app

**When to use:** a workspace crate directly under `crates/frontend/workspaces/`, a standalone tool
that mounts full screen from its own routes. The
[README standard](/documentation/standards/readme_standard.md) defines every rule this template
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

- [<document title>](/documentation/crates/frontend/workspaces/<workspace>/<doc>.md) — <what it covers>
````

## Worked sample

Written from `crates/frontend/workspaces/mission_creator_workspace/src/`. The sample sits in a fenced block, so no
gate reads it as a README; the folder's own README.md is written from the same code and may differ.

````markdown
# Mission Creator

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): the top-down 2D CAD
workspace in which mission makers build a [mission](/documentation/glossary/g_to_m.md#mission) on the
map. This folder holds the editor page, the chrome docked around the map, the canvas mount and its
overlays, the interactive map tools, the loadout editor, the browser session they all run in, and
the read-only review workspace that opens the editor on a submitted version.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/
├── arsenal/           the loadout editor: loadout rows, compatibility, asset catalog, paper doll
├── bridge/            the engine seam: boot, document host, viewport, overlays, tactical graphics
├── input/             pointer and keyboard events turned into map-engine commands; the map tools
├── mission_editor/    the page's parts: canvas mount, registries, placement, transform, toolbar
├── mission_editor.rs  `MissionEditorPage`, which mounts the canvas and raises the chrome around it
├── mod.rs             the module tree and the workspace's contract
├── review_workspace/  the Mission Creator, read-only, on the version an artifact compiled from
├── session/           the browser session: drafts, hydrate, tab lock, review mode, preferences
├── tests/             the page's test suite, mounted from `mission_editor.rs`
└── ui/                docks, top strip, toolbelt, outliner, inspectors, Arsenal panels and dialogs
```

## How it works

`MissionEditorPage` mounts the canvas, boots the engine through `bridge/`, hydrates the mission
document from the server and the local draft (`session/`), loads the item registry, and raises the
docks, toolbelt and overlays around the map. Pointer and keyboard input (`input/`) and every panel
under `ui/` change the document only through the map engine's hosted editing commands, which
`bridge/` hosts together with the document handle, the undo history and the selection; the panels
read signals and never write the document themselves. A module that touches `web_sys` or a live
engine handle compiles for `wasm32` only, so the native test build still compiles the pure half of
the workspace. When `review_workspace/` opens an artifact's version, `session/`'s review mode holds
it and every write path refuses.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | full-bleed, chromeless |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` in `review_workspace/`, which mounts `MissionEditorPage` in review mode | `mission_maker` | full-bleed, chromeless |

## Public surface

- `mission_editor::MissionEditorPage` and `review_workspace::ReviewWorkspacePage`: the route
  components the route table mounts.
- `session::hydrate::purge_local_documents`: drops an account's local drafts; the application root
  registers it as the auth store's sign-out hook.

## Boundaries

- Depends on: `crate::foundation` (the API client and DTOs, the auth store, the UI primitives and
  utilities, the test support), `crate::features` (the review wording), the editing crates of
  `crates/mission_editing/`, the map crates (`map_renderer`, `gpu_frame`, `map_streaming_host`,
  `map_streaming_model`, `map_asset_loading`, `map_render_diagnostics`, `paper_doll_renderer`)
  and `web_sys` in the browser build.
- Used by:
  - `crates/frontend/shell/frontend_application/src/app_routes.rs`, the route table;
  - `crates/frontend/shell/frontend_application/src/main.rs`, which registers `purge_local_documents` as a sign-out hook;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`, which drive the
    `/missions/:id/edit` route.
- Rules: a document mutation goes through the hosted commands of `mission_editing_commands`,
  never straight out of a panel; a module that touches `web_sys` or a live engine handle is `#[cfg(target_arch = "wasm32")]`,
  and so is its `pub mod` line; no page, feature, foundation module or sibling workspace imports
  from this folder.

## Related documentation

- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) — the
  feature inventory, the UX specification, the decisions and the roadmap.
- [Editor gates runbook](/documentation/runbooks/editor_gates.md) — running the headless editor
  gates.
````
