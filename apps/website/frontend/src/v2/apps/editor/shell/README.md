# Browser Session (`v2/apps/editor/shell`)

Everything whose lifetime is the browser tab rather than the document: the IndexedDB draft writer
and its status surface, the server hydrate and the way back from it, the cross-tab writer role,
the warm-session marker, the title preference, the chrome and world-layer preferences, the
payload-size readout, and the transport behind the Save, Export and clipboard commands.

**Depended on by:** `mission_editor.rs` and the top strip, which raise the session surfaces, and
the layout constants the docks and the select gates are both measured against.

**Boundary:** nothing here draws a frame and nothing here decides what a command means — the
policy behind Save, Export and the clipboard belongs to `website_map_engine::editing::commands`.
Everything this directory owns dies with the tab.
