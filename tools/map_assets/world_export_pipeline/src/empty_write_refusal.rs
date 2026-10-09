//! The guard that keeps a stage from replacing committed map data with an empty set.
//!
//! **Role:** [`refuse_empty_write`], called by every emitter before it overwrites a committed
//! artifact.
//! **Position:** the builders, emitters, census and reclassification of this crate call it; nothing
//! outside the crate does.
//! **Signals & state:** none; a pure check.
//! **Invariants:** an empty set is refused with the stage named, before anything is written.

use crate::error::{Result, refuse};

/// Refuses a structurally empty or vacuous overwrite of committed map assets: `empty` is the
/// stage's own verdict, `context` names the stage and `detail` says what was empty.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        refuse!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}
