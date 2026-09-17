# Diagnostics Testbenches (`v2/apps/debug`)

URL-only benches that drive one engine subsystem in isolation, with none of the editor's document,
persistence or chrome around it: the building-blueprint viewer at `/debug/building-viewer`, which
loads a single extracted prefab and probes line of sight and the viewshed through it, and the
world-occluder bench at `/debug/world-los`, which loads the committed object catalogue around a
map point and probes one segment through it. The interior plan lanes both benches draw — walls,
doors, glazing, furniture, vegetation — live here beside them.

**Depended on by:** `app_routes.rs`. Nothing links to these routes from the navigation.

**Boundary:** a bench reads committed assets and engine code only. It never writes a mission
document, never persists, and never imports from a sibling workspace. Every parameter of a run is
in the URL, so a reading reproduces exactly.
