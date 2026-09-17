# Hosted Document (`v2/apps/editor/bridge/document_host`)

The `MissionDocCore` the open mission lives in — its lifecycle and read-only smoke bridge — and
the app-side driver for the undo stack the core itself keeps, with the post-change rebind that
puts the renderer back in step with the document.

**Depended on by:** the top strip's undo and redo buttons, the window keydown dispatch, the save
and hydrate paths, and the headless harness — all through this one driver.

**Boundary:** there is no second undo stack. The core owns the `yrs` undo manager scoped to the
local origin, so only operator gestures are undoable and a seeded or hydrated document is not.
