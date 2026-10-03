//! Why a mission version does not compile into a game document.
//!
//! **Role:** the error of [`crate::flatten_to_mod_document`] and of every entry point built on it.
//! **Position:** returned through [`Result`]; the API's compile and artifact code answers `409`
//! for [`Error::NoSlots`] and records [`Error::Parse`] as the refusal's detail.
//! **Signals & state:** none.
//! **Invariants:** the `Display` texts are the API's stored refusal wording and stay as written.

/// Why a mission version does not compile into a game document.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The payload places no slot, so the game document would carry no playable seat.
    #[error("mission version has no placed slots")]
    NoSlots,
    /// The payload does not parse into the editor graph, or a vehicle names an alias the kit table
    /// does not hold; the text says which.
    #[error("parse mission version payload: {0}")]
    Parse(String),
}

/// The result of a game-document compile.
pub type Result<T> = std::result::Result<T, Error>;
