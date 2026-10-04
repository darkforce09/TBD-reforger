//! The crate's error type.
//!
//! **Role:** [`Error`], why a bench cannot start a run from its URL and the catalog list, and
//! [`Result`].
//! **Position:** returned by the ballistics agreement bench's pure half
//! ([`crate::ballistics_agreement::bench_query`]); the browser half writes the sentence into the
//! bench's status line.
//! **Signals & state:** none; plain values.
//! **Invariants:** each variant's `Display` is the sentence the status line shows, word for word.

/// Why a bench run cannot start.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A URL parameter is present but malformed; the sentence names the parameter and its value.
    #[error("{0}")]
    MalformedBenchQuery(String),
    /// The catalog list is empty, or names neither the requested catalog nor its version; the
    /// sentence names what is missing.
    #[error("{0}")]
    CatalogNotListed(String),
}

/// The crate's result type, failing with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
