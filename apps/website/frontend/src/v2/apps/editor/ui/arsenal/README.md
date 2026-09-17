# Arsenal Panels (`v2/apps/editor/ui/arsenal`)

The views the loadout editor raises inside its modal: the per-container cargo editor, the paper-
doll host that previews the result in 3D with an SVG fallback, and the compatibility and
attachment readouts beside the pick rows.

**Depended on by:** `v2/apps/editor/arsenal`, which mounts them. Nothing else calls them.

**Boundary:** a panel renders and reports its edit through the callback it was handed. The loadout
domain it draws, and the document write, belong to `v2/apps/editor/arsenal`.
