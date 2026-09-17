# Editor Context (`v2/apps/editor/bridge/host_state/editor_context`)

The context installed once at load: the document, render-engine and selection handles every panel
reaches the open mission through, the Leptos signals that mirror the document into the docks, and
the side signals a panel registers when it mounts (asset picker, comment editor, Connections
panel, connection selection).

**Depended on by:** every surface in the workspace that reads the open mission, and the hydrate
and boot paths that install and tear it down.

**Boundary:** every entry point opens exactly one borrow of the context, and any document borrow
is scoped to drop before the post-edit tail takes its own read borrows.
