//! Why an editing command refuses.
//!
//! **Role:** the error of the crate's two fallible commands,
//! [`crate::hosted_commands::orbat_roster::orbat_apply_faction`] and
//! [`crate::document_text::export_text::compiled_export_text`].
//! **Position:** returned through [`Result`]; the Mission Creator shows its `Display` text to the
//! author verbatim.
//! **Signals & state:** none.
//! **Invariants:** a refusal writes nothing to the document; every `Display` text is one readable
//! sentence the author can act on.

/// Why an editing command refuses.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No editing host is installed: the Mission Creator is not open.
    #[error("No mission editor is open.")]
    NoEditorOpen,

    /// The editing host is installed but holds no mission document.
    #[error("No mission document is loaded.")]
    NoDocumentLoaded,

    /// The document refused to apply a faction library document onto a side; the text names why.
    #[error("{0}")]
    FactionRefused(String),

    /// The compiled mission document is not UTF-8 text.
    #[error("compiled document is not UTF-8: {0}")]
    CompiledDocumentNotUtf8(#[from] std::string::FromUtf8Error),
}

/// The result of a fallible editing command.
pub type Result<T> = std::result::Result<T, Error>;
