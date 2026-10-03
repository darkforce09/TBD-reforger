# Full-screen workspaces

The full-screen applications the web app hosts beside its routed pages: the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the debug benches, and two
folders reserved for workspaces that hold no code. Each workspace mounts its own canvas, drives
the map engine directly and owns its whole surface: docks, toolbelts, dialogs, inspectors and the
canvas mount.

## Contents

```text
apps/frontend/src/workspaces/
├── aar/      reserved for the after-action review workspace; holds no code
├── debug/    the URL-only benches: the building viewer and the world line-of-sight bench
├── editor/   the Mission Creator, the top-down 2D CAD workspace in which missions are built
├── mod.rs    the module tree
└── planner/  reserved for the mission planner workspace; holds no code
```

## How it works

`apps/frontend/src/app_routes.rs` mounts each workspace's route component, and
`apps/frontend/src/foundation/route_table/mod.rs` declares every workspace route full-bleed and chromeless, so
it renders without the sidebar and the top bar:

| Workspace | Routes | Access |
|---|---|---|
| `editor/` | `/missions/:id/edit`, and `/missions/:id/artifacts/:artifact_id/workspace` through its review workspace page | `mission_maker` |
| `debug/` | `/debug/building-viewer`, `/debug/world-los`, `/debug/ballistics-agreement` | `none`; no navigation entry |
| `aar/`, `planner/` | no route; `mod.rs` declares no module for them | not routed |

A workspace creates its own `RenderEngine` from `map_renderer` and reaches the GPU crates
only through the map renderer. The [mission](/documentation/glossary/g_to_m.md#mission)
document lives in the map engine's store, which the Mission Creator hosts in `editor/bridge/`; a
workspace keeps the view and session state around it. Code that touches `web_sys` or a live engine
handle compiles for `wasm32` only, so the native test build covers each workspace's pure half.

## Public surface

- The route components `apps/frontend/src/app_routes.rs` mounts:
  `editor::mission_editor::MissionEditorPage`, `editor::review_workspace::ReviewWorkspacePage`,
  `debug::building_viewer::BuildingViewerPage`,
  `debug::world_los::WorldLosPage` and `debug::ballistics_agreement::BallisticsAgreementPage`.
- The Mission Creator's session helpers, which pages and `crate::foundation` reuse; the
  [editor README](/apps/frontend/src/workspaces/editor/README.md) lists them.

## Boundaries

- Depends on: `crate::foundation` (the Mission Creator only; the debug benches use none of it),
  `crate::features` (the review workspace's review wording), the map crates (`map_renderer`,
  `map_streaming_host`, `map_streaming_model`), and the browser bindings in the browser build.
- Used by:
  - `apps/frontend/src/app_routes.rs`, which routes to the components above;
  - the mission library in `apps/frontend/src/pages/mission_hub/library/`, whose upload panel
    reuses the Mission Creator's payload-size formatter;
  - `crate::foundation`: the auth store and the search box, select and slider reuse the Mission
    Creator's `session` module;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`.
- Rules: a workspace imports from `crate::foundation`, `crate::features` and the map crates, never
  from a page or a sibling workspace (`cargo xtask verify frontend-layering`); a workspace's module line in `mod.rs` carries the same `cfg`
  gate as the code it declares; a route added for a workspace needs its row in
  `apps/frontend/src/foundation/route_table/mod.rs` with a recognised tier
  (`every_route_declares_a_recognised_tier` in
  `apps/frontend/src/foundation/route_table/tests/route_authorization.rs`).

## Related documentation

- [Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md) — the
  Mission Creator's roadmap, specifications and decisions.
- [Building viewer](/documentation/apps/frontend/workspaces/debug/building_viewer_page.md) — the
  building bench's purpose and behaviour.
