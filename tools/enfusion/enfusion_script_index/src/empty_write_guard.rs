//! The guard that keeps a committed index from being overwritten with an empty one.
//!
//! **Role:** refuses a write whose result is structurally empty (no symbols, no files, no
//! classes, no carved blobs, no reconstructed sources) before the writer touches its output.
//! **Position:** called by [`crate::index`], [`crate::apidoc`], [`crate::carve`] and
//! [`crate::source`] right before they write.
//! **Signals & state:** none; a pure check.
//! **Invariants:** an empty result is an [`crate::Error::RefusedEmptyWrite`] naming the output and
//! the reason; a non-empty one passes untouched.

use crate::{Error, Result};

/// Refuse a structurally empty write before it overwrites a committed index.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        return Err(Error::RefusedEmptyWrite {
            context: context.to_string(),
            detail: detail.to_string(),
        });
    }
    Ok(())
}
