//! The outcome of one document read.
//!
//! **Role:** `DocumentOutcome` (rendered Markdown or a named raw-text fallback) and
//! `LoadedDocument`, the outcome tagged with the path that was read.
//! **Position:** part of `crate::document_viewer::models`; built by the worker read in
//! `document_loading` and landed into `ViewerState`.
//! **Signals & state:** none; plain data.
//! **Invariants:** a fallback always carries the reason the document could not render.

/// What one read produced — the two terminal states of the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentOutcome {
    /// The document read as UTF-8 within the size cap, for Markdown rendering.
    Rendered {
        /// The document text.
        text: String,
    },
    /// The raw text to show instead, with the note naming why.
    Fallback {
        /// The text that could be read, possibly empty.
        text: String,
        /// Why the document is not rendered: a refusal, a read failure, non-UTF-8 or oversize.
        note: String,
    },
}

/// Worker-thread result: the outcome tagged with the path it answers, so
/// [`super::ViewerState::land`] can drop stale reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDocument {
    /// The repository-relative path the read answers.
    pub rel: String,
    /// What the read produced.
    pub outcome: DocumentOutcome,
}
