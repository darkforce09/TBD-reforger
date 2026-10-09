//! The Mission Creator's browser session: what a tab owns, what it saves, what it restores.
//!
//! **Role:** owns everything whose lifetime is the browser tab rather than the document — the
//! IndexedDB draft writer and its status surface, the server hydrate, its conflict prompt and the
//! way back from it, the cross-tab writer role, the warm-session marker, the title preference the
//! wire carries, the payload-size readout, and the browser transport behind the Save, Export and
//! clipboard commands.
//! **Position:** the third Mission Creator crate: above `mission_creator_engine_bridge`,
//! `mission_creator_state` and the foundation crates; below the Arsenal and the workspace, which
//! mount the conflict prompt and the tab-lock banner, call the commands and register the compile
//! findings publisher. It reads the hosted document through the bridge and writes storage, the
//! network and the clipboard; nothing here draws a frame and nothing here decides what a command
//! means. The app registers `hydrate::purge_local_documents` as a sign-out hook.
//! **Signals & state:** the save status, the tab role and peer count, the conflict and semver
//! signals, the snapshot cache and the collapse latches are all tab-local — they die with the tab.
//! What an operator authored reaches the document through the editing crates instead.
//! **Invariants:** a module that touches `web_sys` or a live document handle is
//! `#[cfg(target_arch = "wasm32")]` and its `pub mod` line carries the same gate, so the native
//! test build still compiles the pure half of the session — the save policy, the writer election
//! and the size arithmetic are all decidable with no browser and are tested that way.

/// The registered publisher the compiled export hands its compile findings to; the workspace's
/// validation panel fills it at page mount.
pub mod compile_findings_publisher;
/// The local-versus-server conflict prompt the hydrate raises: keep the local copy or load the
/// server's.
#[cfg(target_arch = "wasm32")]
pub mod conflict_dialog;
/// The browser transport behind Save, Export and the clipboard commands: the authed POST, the file
/// download, the clipboard write, the toast, the merge report and the smoke bridge the headless
/// harness drives. What a command decides belongs to `mission_editing_commands::document_text`.
pub mod document_commands;
/// The crate's error: why a document command could not hand the operator its file.
pub mod error;
/// The server hydrate: the authed `GET /missions/:id`, the local-versus-server conflict prompt,
/// and the snapshot pair that is the way back from an adopt.
#[cfg(target_arch = "wasm32")]
pub mod hydrate;
/// The toolbelt's payload-size estimate for the mission as it would compile.
pub mod mission_size;
/// The IndexedDB draft writer: the per-account record store and the debounced, serialized-per-
/// mission save that never clobbers a good record.
#[cfg(target_arch = "wasm32")]
pub mod persist;
/// The items most callers name, for `use mission_creator_session::prelude::*;`.
pub mod prelude;
/// The observable autosave status the writer reports into: the status value, the chip that renders
/// it and the one toast per failed episode.
pub mod save_status;
/// Cross-tab presence for one mission: the writer role a tab holds, the read-only role a second
/// tab takes, and the save-decision policy `persist` obeys.
pub mod tab_lock;
/// The warm-editor-session marker in `sessionStorage`, scoped to the signed-in account.
#[cfg(target_arch = "wasm32")]
pub mod warm_session_marker;
