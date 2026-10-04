//! The crate's error: why a body of review text cannot be sent.
//!
//! **Role:** the one failure type of the crate's fallible public function,
//! [`crate::review_wording::validated_review_text`].
//! **Position:** built where review text is checked before a comment, approval conditions or a
//! rejection reason is sent; read by the comment composer and the approvals decision form, which
//! show it under the text box.
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant's `Display` is its sentence byte for byte, the line the reviewer and
//! the author are shown; no caller branches on it.

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why a body of review text cannot be sent.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The text is blank, or longer than the backend's 8000 bytes: `Write the <what> first`, or
    /// `The <what> is too long: <n> bytes, at most 8000`.
    #[error("{0}")]
    InvalidReviewText(String),
}
