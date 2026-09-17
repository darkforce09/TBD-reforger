# Mission Planner (`v2/apps/planner`)

An empty scaffold. The pre-match tactical whiteboard will live here: drawing operational plans —
phase lines, assault arrows, boundaries, squad tasking and the commander's intent — over a
published scenario, for briefing rather than authoring.

**Depended on by:** nothing yet. `v2/apps/mod.rs` declares only `debug` and `editor`, so no code
in this directory is compiled, and `app_routes.rs` carries no planner route.

**Boundary, and the reason it is written down before there is code:** the planner will consume the
same `website-map-engine` the editor does — its camera, world streaming, symbology, picking and
`editing/` tool state machines. `editing/` must therefore stay free of editor-only assumptions:
that a mission document is open for authoring, that an undo stack exists, that a palette arm is
pending. A tool that can only run inside the Scenario Creator blocks this workspace before it is
written.
