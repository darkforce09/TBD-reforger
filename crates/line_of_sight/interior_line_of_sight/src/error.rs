//! Why an interior line-of-sight request is refused.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the
//! [`ViewshedCapRefused`] a floor wash over the radius cap reports.
//! **Position:** converted into with `?` from the cap check of [`crate::floor_wash`].
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant wraps the refusal transparently: the message names the cap, its
//! limit and the measured radius.

use terrain_line_of_sight::viewshed::ViewshedCapRefused;

/// Why an interior line-of-sight request is refused.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum Error {
    /// The floor wash's radius exceeds the cap.
    #[error(transparent)]
    CapRefused(#[from] ViewshedCapRefused),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
