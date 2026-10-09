//! The guard that keeps a lane from replacing a committed map asset with an empty one.
//!
//! **Role:** [`refuse_empty_write`], called by the stitch, the glyph atlas, the height labels and
//! both archive emitters before they overwrite a committed asset.
//! **Position:** the lanes of this crate call it; nothing outside the crate does.
//! **Signals & state:** none; a pure check.
//! **Invariants:** an empty result is refused with the lane named, before anything is written.

use crate::error::{Result, bail};

/// Refuses a structurally empty or vacuous overwrite of committed map assets: `empty` is the
/// lane's own verdict, `context` names the lane and `detail` says what was empty.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}
