//! The browser session the editor runs inside: what a tab owns, what it saves, what it restores.
//!
//! **Role:** owns everything whose lifetime is the browser tab rather than the document — the
//! IndexedDB draft writer and its status surface, the server hydrate and the way back from it, the
//! cross-tab writer role, the read-only review workspace mode, the warm-session marker, the title
//! preference the wire carries, the chrome layout preferences and world-layer preferences a person
//! keeps, the payload-size readout, and the browser transport behind the Save, Export and clipboard
//! commands.
//! **Position:** a layer under the editor workspace. It reads the hosted document through the
//! bridge and writes storage, the network and the clipboard; nothing here draws a frame and
//! nothing here decides what a command means.
//! **Signals & state:** the save status, the tab role and peer count, the conflict and semver
//! signals, the snapshot cache and the collapse latches are all tab-local — they die with the tab.
//! What an operator authored reaches the document through `website_map_engine::editing` instead.
//! **Invariants:** a module that touches `web_sys` or a live document handle is
//! `#[cfg(target_arch = "wasm32")]` and its `pub mod` line carries the same gate, so the native
//! test build still compiles the pure half of the session — the save policy, the writer election,
//! the size arithmetic and the layout constants are all decidable with no browser and are tested
//! that way.

/// The browser transport behind Save, Export and the clipboard commands: the authed POST, the file
/// download, the clipboard write, the toast, the merge report and the smoke bridge the headless
/// harness drives. What a command decides belongs to `website_map_engine::editing::commands`.
pub mod document_commands;
/// The docked chrome's single import path — re-exports the dialog and panel components under
/// [`super::ui`] so consumers name one module rather than tracking which surface file holds
/// which component.
pub mod eden_chrome;
/// The server hydrate: the authed `GET /missions/:id`, the local-versus-server conflict prompt,
/// and the snapshot pair that is the way back from an adopt.
#[cfg(target_arch = "wasm32")]
pub mod hydrate;
/// The chrome inset constants and shared class recipes the strip, docks and toolbelt are laid out
/// from. [`super::input::tools::select_tool`] and [`super::mission_editor`] read the same
/// constants back, so the pan, select and marquee gates stay aligned with whatever the panels
/// currently occupy.
pub mod layout;
/// The toolbelt's payload-size estimate for the mission as it would compile.
pub mod mission_size;
/// The IndexedDB draft writer: the per-account record store and the debounced, serialized-per-
/// mission save that never clobbers a good record.
#[cfg(target_arch = "wasm32")]
pub mod persist;
/// The read-only review workspace: the reviewed version an editor mount shows, and the one
/// predicate every write path consults before writing the mission.
pub mod review_mode;
/// The observable autosave status the writer reports into: the status value, the chip that renders
/// it and the one toast per failed episode.
pub mod save_status;
/// The warm-editor-session marker in `sessionStorage`, scoped to the signed-in account.
#[cfg(target_arch = "wasm32")]
pub mod session;
/// Cross-tab presence for one mission: the writer role a tab holds, the read-only role a second
/// tab takes, and the save-decision policy [`persist`] obeys.
pub mod tab_lock;
/// The anti-stomp rule that decides which title an adopt carries, and the pins that hold the
/// mission row's metadata wire together across the engine wall.
pub mod title_prefer;
/// Per-user world-layer visibility and basemap preferences, persisted to local storage. The wasm
/// host applies them to the chunk residency and the engine on each settle.
pub mod world_layer_prefs;
