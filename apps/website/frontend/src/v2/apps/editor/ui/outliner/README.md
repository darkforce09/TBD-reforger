# Editor Layers Outliner (`v2/apps/editor/ui/outliner`)

The hierarchical view of the document's editor layers and the slots filed under them: the node
model, the windowed renderer the docks draw with, and the pointer-drag latch that refiles rows.

**Depended on by:** the left dock, the right dock's palette, and the ORBAT manager dialog.

**Boundary:** a row action reaches the document through the map engine's layer operations, and
every drag arm builds a set the drop consumes, so a multi-row drag can never move its anchor
alone. Rows are derived from the document on every rebuild and are never cached.
