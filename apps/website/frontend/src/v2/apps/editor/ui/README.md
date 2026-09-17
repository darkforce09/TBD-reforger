# Editor Surfaces (`v2/apps/editor/ui`)

Every pixel the operator sees around the map, grouped by the surface it draws: the chrome docked
around the viewport, the layer outliner those docks host, the inspectors that edit the selected
subject, the Arsenal's loadout panels, and the full-screen dialogs raised over all of it.

**Depended on by:** `mission_editor.rs`, which mounts them, and `shell/eden_chrome`, which
re-exports the chrome components under one import path.

**Boundary:** a surface renders and dispatches; it never mutates the document directly. A view
that touches `web_sys` gates the touching body rather than the whole component, so the native test
build still compiles the surface.
