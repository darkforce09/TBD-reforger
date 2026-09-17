# Scenario Creator (`v2/apps/editor`)

The 2D/3D CAD workspace a mission is authored in: the editor page, the chrome docked around the
map, the canvas mount and its overlays, the interactive map tools, the arsenal loadout editor, and
the browser session all of it runs inside.

```text
editor/
├── mission_editor.rs   the route component that mounts the canvas and raises the chrome
├── arsenal/            loadout domain — rows, compatibility rules, asset catalog, 3D doll
├── bridge/             the engine seam — canvas mount, boot, viewport, overlays, host state
├── input/              DOM pointer and keyboard events turned into map-engine commands
├── shell/              the browser session — drafts, hydrate, tab locking, preferences
├── ui/                 the rendered surfaces — docks, outliner, inspectors, modals, arsenal panels
└── tests/              sibling test files mounted by `mission_editor.rs`
```

**Depended on by:** `app_routes.rs`, which mounts it full screen from the mission routes.

**Boundary:** a document mutation travels through `website_map_engine::editing`, never straight
out of a panel. Nothing under `v2/pages` reaches in, and no sibling workspace does either.
