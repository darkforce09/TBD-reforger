# Standalone Workspaces (`src/v2/apps`)

The full-screen applications the SPA hosts beside its document pages. Each mounts its own canvas
and drives the map and graphics engines directly, owning its entire surface — docks, toolbelts,
modals, inspectors and canvas mounting.

```text
apps/
├── editor/    Scenario Creator — the 2D/3D CAD workspace a mission is authored in
├── planner/   Mission Planner — tactical whiteboard and briefing interface (scaffold)
├── aar/       After-Action Report — telemetry replay player (scaffold)
└── debug/     Engine diagnostics testbenches — building viewer and world line-of-sight
```

**Depended on by:** `app_routes.rs`, which routes each workspace full screen, and the pages that
link to them.

**Boundary:** a workspace imports from `v2/core` and from the engine crates (`website-map-engine`,
and the graphics engine only through it). It never imports from `v2/pages` and never from a
sibling workspace. Document state belongs to the map engine; only what dies with the browser tab
lives here.
