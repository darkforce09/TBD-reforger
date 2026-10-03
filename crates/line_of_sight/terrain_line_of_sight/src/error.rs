//! Why a terrain line-of-sight request is refused.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the
//! [`ViewshedCapRefused`] a viewshed over the cell cap reports.
//! **Position:** converted into with `?` from the cap check of [`crate::viewshed`] and
//! [`crate::viewshed_job`].
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant wraps the refusal transparently: the message names the cap, its
//! limit and the measured value.

use crate::viewshed::ViewshedCapRefused;

/// Why a terrain line-of-sight request is refused.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum Error {
    /// The viewshed's raster would hold more cells than the cap allows.
    #[error(transparent)]
    CapRefused(#[from] ViewshedCapRefused),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
