# Docked Chrome (`v2/apps/editor/ui/docks`)

The five surfaces that frame the canvas: the left dock (Editor Layers outliner and named
locations), the right dock (the Factions / Vehicles / Zones / Markers palette), the top command
strip, the bottom toolbelt with its mode toolbar and status read-outs, and the right-click context
menu.

**Depended on by:** `mission_editor.rs`, and `shell/eden_chrome`, which re-exports them.

**Boundary:** the docks are ungated — the bodies that drive the document are gated inside their
event closures, so the native view shell compiles them too. Their insets come from `shell/layout`,
the same constants the pan, select and marquee gates read.
