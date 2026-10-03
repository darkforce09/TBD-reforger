//! Source-text pins of the frame engine's mission lane bind functions.
//!
//! **Role:** reads the engine, upload, bridge, diagnostics and frontend history sources as text
//! and pins what the comment, connection and symbology bind functions upload and what they leave
//! alone.
//! **Position:** test-only, mounted from `crate::frame`; every case `include_str!`s the files it
//! pins, so the suite compiles on every target.
//! **Signals & state:** none.
//! **Invariants:** a bind function uploads its own lane and never touches the pick bridge's
//! `last_ids`; both document rebinds feed `comments_bind`.

#[path = "history_rebind_feeds_comments.rs"]
mod history_rebind_feeds_comments;

#[path = "comments_bind_skips_pick_bridge.rs"]
mod comments_bind_skips_pick_bridge;

#[path = "connections_bind_skips_pick_bridge.rs"]
mod connections_bind_skips_pick_bridge;

#[path = "symbology_bind_paths.rs"]
mod symbology_bind_paths;
