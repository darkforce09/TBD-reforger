# Input Layer (`v2/apps/editor/input`)

Every browser event the operator generates over the map, turned into an intent the rest of the
editor acts on: the pointer, wheel, context-menu and double-click gestures over the canvas, the
two window-level keydown dispatches, and the browser half of the interactive map tools.

**Depended on by:** `mission_editor.rs`, which installs the gesture context and the window
listeners when the canvas mounts.

**Boundary:** this layer decides nothing a document must remember. It reads the live handles
`bridge/` holds, flips host signals, and writes through `website_map_engine::editing`. Undo and
redo have exactly one entry point — `bridge/document_host/history` — which it calls rather than
stepping the stack itself.
