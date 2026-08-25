//! T-934.6 — in-memory reactive editor state & document commands. wasm32 gates
//! reproduced from the pre-reorg main.rs declarations exactly.

// T-159.20 Save Version + Export — compile (map-engine-core `mission`) + authed POST + file
// download. Ungated so T-417 Class-R helpers/tests compile on native `cargo test`; the wasm
// transport body lives behind `#[cfg(target_arch = "wasm32")]` inside the file (sse.rs pattern).
// (mission_commands.rs before the T-934.6 rename; audit's editor gesture commands land as
// `commands.rs` in the B2 phase.)
pub mod commands_hotkeys;
// T-159.16 MissionDoc host — all content is wasm32-only (links map-engine-core `doc`).
#[cfg(target_arch = "wasm32")]
pub mod doc_host;
// T-159.21 undo/redo — drives the hosted MissionDocCore undo stack (+ the post-change glyph
// rebind and the `__editorHistory` bridge); wasm32-only.
#[cfg(target_arch = "wasm32")]
pub mod history;
// T-159.26 server hydrate / conflict / dirty — GET /missions/:id → hydrate the saved version or
// prompt on a local-vs-server conflict. wasm32-only (auth GET + doc).
#[cfg(target_arch = "wasm32")]
pub mod hydrate;
// T-159.22 dock commands — outliner select / active layer / palette drag-to-place. Drives the
// hosted MissionDocCore, so wasm32-only. (editor_ops.rs before the T-934.6 rename; becomes a
// façade over state/ops/ at T-934.7.)
#[cfg(target_arch = "wasm32")]
pub mod operations;
// T-159.17 yrs IDB persist — IndexedDB (`idb` crate) + debounced writer; wasm32-only.
#[cfg(target_arch = "wasm32")]
pub mod persist;
// T-159.17 warm editor session — sessionStorage marker; wasm32-only (web-sys/js-sys).
#[cfg(target_arch = "wasm32")]
pub mod session;
// T-522 — prefer-payload anti-stomp Class-R must run on native `cargo test`. The live hydrate
// module stays wasm32-gated; the pure prefer helper + t505 pin live here.
pub mod title_prefer;
