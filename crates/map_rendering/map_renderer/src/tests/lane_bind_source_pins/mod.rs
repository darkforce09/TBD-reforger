//! Source-text pins of the render engine's mission lane bind functions.
//!
//! **Role:** reads the slot symbology layer and hairline upload sources that define the bind
//! functions as text and pins what the comment, connection and symbology bind functions upload
//! and what they leave alone.
//! **Position:** test-only, mounted from the crate root; every case `include_str!`s the files it
//! pins, so the suite compiles on every target.
//! **Signals & state:** none.
//! **Invariants:** a bind function uploads its own lane and never touches the pick bridge's
//! `last_ids`.

#[path = "comments_bind_skips_pick_bridge.rs"]
mod comments_bind_skips_pick_bridge;

#[path = "connections_bind_skips_pick_bridge.rs"]
mod connections_bind_skips_pick_bridge;

#[path = "symbology_bind_paths.rs"]
mod symbology_bind_paths;
