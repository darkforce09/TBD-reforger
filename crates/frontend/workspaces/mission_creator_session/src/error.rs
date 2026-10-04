//! The crate's error: why a document command could not hand the operator its file.
//!
//! **Role:** the one failure type of the crate's fallible public functions, the compiled export
//! (`document_commands::compiled_document_json` and its diagnostics form) and the JSON download
//! (`document_commands::download_json`).
//! **Position:** built where a compile of the live document refuses or the browser refuses a step
//! of the download; read by the Export Compiled and Export JSON commands, the `__editorCommands`
//! smoke bridge and the Arsenal's loadout export, which show the text in a toast or drop it.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant's `Display` is the operator-facing text byte for byte (a compile
//! refusal as the compile worded it, a download refusal as the browser's error value prints in
//! debug form); no caller branches on the variant.

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why a document command could not hand the operator its file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The live document did not compile: the mission row never arrived, the editor is not
    /// mounted, or the compiler refused. The text is the refusal as the toast shows it.
    #[error("{0}")]
    CompileRefused(String),
    /// The browser refused a step of the file download (no window or document, the blob, the
    /// object URL or the anchor). The text is the browser's error value in debug form.
    #[error("{0}")]
    DownloadRefused(String),
}
