//! Reclaim orphan build caches. THIS IS NOT OPTIONAL HOUSEKEEPING — it is the failure that stopped
//! this program dead once.
//!
//! OBSERVED 2026-07-26: the disk hit **252 MB free of 952 GB** mid-wave. Two gate steps failed with
//! "No space left on device", which reads exactly like a build error. `/var/tmp` held ~116 GB of
//! agent target dirs from slices that had already SHIPPED — every agent is told to remove its own
//! and many either forgot or were killed by a session limit before they could.
//!
//! Skips any dir belonging to a slice whose worktree still exists, so a live agent's cache
//! survives.
//!
//! Gate-private dirs (`target-gate-*`, `dist-gate-*`) live at MAIN_ROOT, not `/var/tmp` —
//! ~15 GB class, expensive to rebuild, warm is valuable (measured cold 23.4 s vs warm 9.3 s
//! slice gate). Default reclaim does NOT touch them; opt in with `--gate-dirs`. Optional
//! `--gate-dirs-older-than-days N` only removes gate dirs whose directory mtime is older than N
//! days (age-based sweep without nuking a cache that was used today).
//!
//! PER-SLICE private dirs (`target-<SLICE>`, `target-<SLICE>-api`) ALSO live at MAIN_ROOT,
//! and nothing else reaps them. See the block inside for why they are swept BY
//! DEFAULT while the gate set stays opt-in — the two look alike and are opposites.

use std::path::{Path, PathBuf};

use super::Ctx;
use crate::{werr, wprintln};

#[cfg(test)]
#[path = "tests/reclaim/tests.rs"]
mod tests;

mod du_mb;
pub use du_mb::cmd_reclaim;

mod adhoc_token;
use adhoc_token::adhoc_token;
use adhoc_token::df_avail;
use adhoc_token::dir_age_days;

#[cfg(test)]
use du_mb::{key_of, slice_token};
