# Full-screen workspaces

The full-screen applications the web app hosts beside its routed pages: the
[Mission Creator](/documentation_v2/glossary.md#mission-creator), the debug benches, and two
folders reserved for workspaces that hold no code. Each workspace mounts its own canvas, drives
the map engine directly and owns its whole surface: docks, toolbelts, dialogs, inspectors and the
canvas mount.

## Contents

```text
apps/website/frontend/src/v2/apps/
├── aar/      reserved for the after-action review workspace; holds no code
├── debug/    the URL-only benches: the building viewer and the world line-of-sight bench
├── editor/   the Mission Creator, the 2D/3D CAD workspace in which missions are built
├── mod.rs    the module tree
└── planner/  reserved for the mission planner workspace; holds no code
```

## How it works

`apps/website/frontend/src/app_routes.rs` mounts each workspace's route component, and
`apps/website/frontend/src/router.rs` declares every workspace route full-bleed and chromeless, so
it renders without the sidebar and the top bar:

| Workspace | Routes | Access |
|---|---|---|
| `editor/` | `/missions/:id/edit`, and `/missions/:id/artifacts/:artifact_id/workspace` through the mission hub's review workspace page | `mission_maker` |
| `debug/` | `/debug/building-viewer`, `/debug/world-los` | `none`; no navigation entry |
| `aar/`, `planner/` | no route; `mod.rs` declares no module for them | not routed |

A workspace creates its own `RenderEngine` from `website_map_engine::frame` and reaches the
graphics engine only through the map engine. The [mission](/documentation_v2/glossary.md#mission)
document lives in the map engine's store, which the Mission Creator hosts in `editor/bridge/`; a
workspace keeps the view and session state around it. Code that touches `web_sys` or a live engine
handle compiles for `wasm32` only, so the native test build covers each workspace's pure half.

## Public surface

- The route components `apps/website/frontend/src/app_routes.rs` mounts:
  `editor::mission_editor::MissionEditorPage`, `debug::building_viewer::BuildingViewerPage` and
  `debug::world_los::WorldLosPage`.
- The Mission Creator's review mode and shell helpers, which pages and `crate::v2::core` reuse; the
  [editor README](/apps/website/frontend/src/v2/apps/editor/README.md) lists them.

## Boundaries

- Depends on: `crate::v2::core` (the Mission Creator only; the debug benches use none of it),
  `website_map_engine`, and the browser bindings in the browser build.
- Used by:
  - `apps/website/frontend/src/app_routes.rs`, which routes to the three components;
  - the mission hub pages in `apps/website/frontend/src/v2/pages/mission_hub/`: the review
    workspace mounts the Mission Creator in review mode, and the library's upload panel reuses its
    payload-size formatter;
  - `crate::v2::core`: the auth store and the search box, select and slider reuse the Mission
    Creator's shell;
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`.
- Rules: a workspace imports from `crate::v2::core` and `website_map_engine`, never from a page or
  a sibling workspace, and no gate checks it; nothing here imports `website_graphics_engine`
  (`cargo xtask verify engine-layers`); a workspace's module line in `mod.rs` carries the same `cfg`
  gate as the code it declares; a route added for a workspace needs its row in
  `apps/website/frontend/src/router.rs` with a recognised tier
  (`every_route_declares_a_recognised_tier` in
  `apps/website/frontend/src/tests/route_authorization.rs`).

## Related documentation

- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  Mission Creator's roadmap, specifications and decisions.
- [Building viewer](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) — the
  building bench's purpose and behaviour.
