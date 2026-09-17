# Engine Seam (`v2/apps/editor/bridge`)

The frontend's side of the engine boundary: the canvas mount and its boot machine, the viewport
and frame-timing belt, the floating overlays laid over the map, the tactical-graphics belt, the
map-asset host, the hosted document with its undo drive, and the host signal state the engine's
hosted commands read.

**Depended on by:** `mission_editor.rs`, which mounts the canvas, and every dock, inspector and
tool that needs the live document — all of which reach it through `host_state` and the command
layers rather than through a handle of their own.

**Boundary:** this is the only place in the editor that holds a live engine, document or host
handle. What the canvas draws and what a click can pick come from one read of the document, never
from two parsers kept in step by hand.
