//! The primitives every API crate builds on.
//!
//! **Role:** the one handler failure and its `{error, details?}` JSON envelope
//! ([`error_handling`]), the JSON wire formats the models share ([`wire_format`]), the HTML
//! sanitiser and the content URL policy of authored text ([`text`]), and the request parameters
//! every list and path route reads ([`http`]).
//! **Position:** the lowest API crate above `api_identifiers`' tier, depending on
//! `content_digest` and external crates only; every other API crate and the API application
//! build on it.
//! **Signals & state:** none; plain values and pure functions.
//! **Invariants:** a database error reaches a client as `internal error`, never as its text; the
//! wire spellings here are contract, pinned by the API's model and golden tests; nothing here
//! names a domain.

mod error;
pub mod error_handling;
pub mod http;
pub mod prelude;
pub mod text;
pub mod wire_format;

pub use error::{Error, Result};
