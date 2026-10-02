//! Role: Module boundary for the editor commands' decidable half.
//! Position: `editing` in the map engine.
//! Signals & state: none; every function here is pure over its arguments.
//! Invariants: what a command DECIDES lives here — the bytes an export writes, the wording a
//! report carries, the rows a selection resolves to. How those bytes reach a disk, a clipboard or
//! a server is the host's, and none of it is named in this module.

/// What an export writes, and what it refuses to write.
pub mod export_text;

/// Phrase a merge report and a duplicate-id report for the author.
pub mod merge_report;

/// Resolve a selection out of the document and phrase it for a clipboard.
pub mod selection_digest;
