//! The crate's error: why a pack listing or a saved-copy read produced nothing usable.
//!
//! **Role:** the one failure type of the crate's fallible public functions:
//! [`crate::offline_manifest::catalog_targets`] and the browser read
//! `crate::saved_copies::read_text`.
//! **Position:** built where the list is parsed and where the read fails; read by the pack
//! download, which logs it behind the listing it names, and by the mortar calculator's catalog
//! source, which shows it as the read's failure.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant's `Display` is its reason byte for byte, the sentence the
//! interface and the download's log show; no caller branches on more than the variant.

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why a pack listing or a saved-copy read produced nothing usable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The catalog list body did not parse, or named a catalog id that does not form a catalog
    /// version path.
    #[error("{0}")]
    CatalogListUnreadable(String),

    /// Neither the server nor a saved copy answered a read: the refusal (`Request failed
    /// (<status>)`) or the browser's own failure message.
    #[error("{0}")]
    ReadFailed(String),
}
