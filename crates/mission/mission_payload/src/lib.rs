//! The mission payload compiler.
//!
//! **Role:** compiles a Mission Creator document's by-id maps into the editor payload a mission
//! version stores ([`compile_payload`]), wraps a payload in the export envelope
//! ([`compile_export`]) and the version body of `POST /missions/:id/versions` ([`version_body`],
//! [`version_body_to_writer`]), stamps the terrain bounds ([`terrain_bounds`]) and holds the kit
//! alias table the game-document compiler resolves registry aliases through ([`kit_aliases`]).
//! **Position:** mission tier 2, over `mission_model` (the ORBAT projection and the authored
//! block registry), `serde` and `serde_json`. The Mission Creator compiles through it on Save and
//! Export; the game-document compiler, the validator and the document operations call it; the
//! API reads the kit aliases.
//! **Signals & state:** the kit alias table is parsed once into a process-wide `OnceLock`;
//! everything else is pure functions.
//! **Invariants:** the payload keys and their order are the editor payload schema's; an authored
//! block rides the environment bag and lands at the payload root verbatim; the kit alias table is
//! `contracts/rules/kit-aliases.json`, embedded at build time.

mod error;
mod export;
pub mod kit_aliases;
pub mod prelude;
mod serialization;
mod terrain_bounds;

/// Why writing a version body failed.
pub use error::Error;
/// The result of a payload write.
pub use error::Result;
/// The `POST /missions/:id/versions` body around a payload, as a JSON value.
pub use export::version_body;
/// The `POST /missions/:id/versions` body around a payload, written straight into a writer.
pub use export::version_body_to_writer;
/// The top-level keys of an editor payload the compiler knows, in schema order.
pub use serialization::KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS;
/// The export envelope around a compiled payload.
pub use serialization::compile_export;
/// The editor payload compiled from a document's by-id maps.
pub use serialization::compile_payload;
/// Whether a key is one of [`KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS`].
pub use serialization::is_known_editor_payload_top_level;
/// The world bounds `[min_x, min_z, max_x, max_z]` of a terrain, in metres.
pub use terrain_bounds::terrain_bounds;

#[cfg(test)]
mod tests;
