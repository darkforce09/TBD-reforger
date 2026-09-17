# Editor Dialogs (`v2/apps/editor/ui/modals`)

The surfaces that overlay the whole workspace rather than frame the map: the controls hint and
shortcut reference, the mission settings sheet, the faction template dialog and the ORBAT graph
dialog.

**Depended on by:** `mission_editor.rs`, which mounts every one of them.

**Boundary:** none of these is routed — they are editor surfaces, never pages. A dialog that binds
a key is censused by the help modal's keymap census, which adjudicates every editor binding
against every other; a dialog stacked over another takes its z from `core::ui::modal_stack` and
gates Escape on being topmost.
