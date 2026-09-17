# Host Signal State (`v2/apps/editor/bridge/host_state`)

What the browser knows and the engine must be told: the editor context installed at load (the
document, render engine and selection handles, and the signals mirroring the document into the
docks), the in-flight placement, the selected-entity set, and the host half of undo grouping.

**Depended on by:** every panel and tool that reads or writes editor state, and by
`website_map_engine::editing`, which reads it back through the host closures it is handed.

**Boundary:** none of this is document state — an arm is never undoable, a selection mints no undo
step, and an unset signal is silence rather than an error. The handles are `!Send` `Rc`s, so the
whole directory is wasm-only and reached through thread-locals.
