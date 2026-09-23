# Browser Session (`v2/apps/editor/shell`)

Everything whose lifetime is the browser tab rather than the document: the IndexedDB draft writer
and its status surface, the server hydrate and the way back from it, the cross-tab writer role,
the read-only review mode, the warm-session marker, the title preference, the chrome and
world-layer preferences, the payload-size readout, and the transport behind the Save, Export and
clipboard commands.

**Review mode** (`review_mode.rs`): the review workspace route opens it on the version an artifact
compiled from before the editor mounts. While it is open the boot restores that version instead of
the draft and the server's current version, and every write path consults it — the draft writer is
never armed, the writer role reads as read-only without an election, the unload prompt stays off,
Save Version refuses, and neither mission-row mirror patches the row. Export Compiled compiles over
the row fields the artifact recorded.

**Depended on by:** `mission_editor.rs` and the top strip, which raise the session surfaces, the
layout constants the docks and the select gates are both measured against, and the review
workspace route, which opens and closes review mode.

**Boundary:** nothing here draws a frame and nothing here decides what a command means — the
policy behind Save, Export and the clipboard belongs to `website_map_engine::editing::commands`.
Everything this directory owns dies with the tab.
