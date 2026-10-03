# frame/tests/lane_bind_source_pins

Source-text pins of the frame engine's mission lane bind functions: each case reads the engine,
upload, bridge and diagnostics sources (and the frontend history host) as text with
`include_str!`, cuts out one function body and asserts what it calls. They compile on every target
because they never construct a `RenderEngine`.

## Contents

- `comments_bind_skips_pick_bridge.rs` — `comments_bind` uploads `MissionComments` and touches
  neither `last_ids` nor `slots_bind_soa`.
- `connections_bind_skips_pick_bridge.rs` — `connections_bind` uploads `MissionConnections` and
  skips the pick bridge.
- `history_rebind_feeds_comments.rs` — both document rebinds of
  `apps/frontend/src/workspaces/editor/bridge/document_host/history.rs` feed `comments_bind`.
- `mod.rs` — the module tree, mounted from `crate::frame`.
- `symbology_bind_paths.rs` — the slot atlas, the slot and vehicle symbology binds and the comment
  lane refresh keep their upload paths.

## Boundaries

A pinned function renamed away or a source file moved fails the case loudly; run the crate suite
with `--all-features`.
