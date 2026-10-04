//! The Mission Creator's workspace: the editor page and everything drawn around the map.
//!
//! **Role:** owns the surfaces the operator drives while editing a mission: the editor page and
//! its canvas mount, the docked chrome around the map (left and right docks, top strip,
//! toolbelt, context menu), the Editor Layers outliner, the inspectors, the full-screen dialogs
//! and the read-only review workspace that opens the editor on a submitted version.
//! **Position:** the fifth and top Mission Creator crate, above `mission_creator_arsenal`,
//! `mission_creator_session`, `mission_creator_engine_bridge`, `mission_creator_state` and the
//! foundation and feature crates; the app's route table mounts its two route components,
//! `MissionEditorPage` and `ReviewWorkspacePage`. At mount the page fills the lower crates'
//! registered hooks: the draft-persist hook, the right-click opener and the compile-findings
//! publisher.
//! **Signals & state:** the document handle, the undo history, the selection and the armed
//! placement live in [`mission_creator_engine_bridge::bridge`]; the session signals live in
//! [`mission_creator_session`]; the page creates its own page signals at mount and every panel
//! reads those signals and writes back through the hosted commands of `mission_editing_commands`.
//! **Invariants:** a document mutation travels through the editing crates, never straight out of
//! a panel. A module that touches `web_sys` or a live engine handle is
//! `#[cfg(target_arch = "wasm32")]`, and its `pub mod` line carries the same gate, so the native
//! test build still compiles the pure half of the workspace; an item only browser code and the
//! tests read carries `#[cfg(any(test, target_arch = "wasm32"))]`.

/// The crate's error: why an inspector or dialog edit was refused.
pub mod error;
/// The editor page itself: the route component that mounts the canvas, raises the chrome around
/// it, and wires the docks, tools and overlays to the document.
pub mod mission_editor;
/// The items most callers name, for `use mission_creator_workspace::prelude::*;`.
pub mod prelude;
/// The read-only review workspace: the Mission Creator opened on a submitted version, routed at
/// `/missions/:id/artifacts/:artifact_id/workspace`.
pub mod review_workspace;
/// The rendered surfaces around the map: the docked chrome (left and right docks, top strip,
/// toolbelt, context menu), the Editor Layers outliner they host, the inspectors that edit the
/// selected subject, and the full-screen dialogs raised over the whole workspace.
pub mod ui;

/// The frontend source trees the whole-frontend source pins walk.
#[cfg(test)]
#[path = "tests/frontend_source_roots.rs"]
mod frontend_source_roots;

/// The Mission Creator page routed at `/missions/:id/edit`.
#[cfg(target_arch = "wasm32")]
pub use mission_editor::MissionEditorPage;
/// The read-only review workspace routed at `/missions/:id/artifacts/:artifact_id/workspace`.
#[cfg(target_arch = "wasm32")]
pub use review_workspace::ReviewWorkspacePage;
