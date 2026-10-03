# Lane bind source pins

Source-text pins of the render engine's mission lane bind functions: each case reads the files
that define the pinned functions (the symbology layers' slot symbology, the hairline upload) as
text with `include_str!`, cuts out one function body and asserts what it calls. They compile on
every target because they never construct a `RenderEngine`.

## Contents

- `comments_bind_skips_pick_bridge.rs` — `comments_bind` uploads `MissionComments` and touches
  neither `last_ids` nor `slots_bind_soa`.
- `connections_bind_skips_pick_bridge.rs` — `connections_bind` uploads `MissionConnections` and
  skips the pick bridge.
- `mod.rs` — the module tree, mounted from the crate root.
- `symbology_bind_paths.rs` — the slot atlas, the slot and vehicle symbology binds and the comment
  lane refresh keep their upload paths, and no SoA bind bypasses `slots_bind_symbology`.

## Boundaries

A pinned function renamed away or a source file moved fails the case loudly; run the crate suite
with `--all-features`. The pin that both document rebinds of the frontend's history host feed
`comments_bind` lives with that host, in
`apps/frontend/src/workspaces/editor/bridge/tests/document_host/history_rebind_feeds_comments.rs`.
