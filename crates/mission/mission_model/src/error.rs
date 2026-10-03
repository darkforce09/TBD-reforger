//! Why a check of the mission model refuses a value.
//!
//! **Role:** the one error type of the crate's fallible functions: the parse and validate
//! functions of every authored block, the task schedule and state checks, the radio frequency
//! check and the ORBAT faction join key check.
//! **Position:** returned through [`Result`] by the block modules; the payload compiler, the
//! game-document compiler, the API and the Mission Creator read its message.
//! **Signals & state:** none.
//! **Invariants:** an error's `Display` text is the exact sentence the check has always produced,
//! so every message the Mission Creator shows and every API error body stays the same.

/// Why a mission model check refuses a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// An authored block, or one field of it, breaks a rule of its schema or of the mod that
    /// reads it; the clause names the field, the value and the rule.
    #[error("{0}")]
    Refused(String),
    /// An ORBAT faction join key is empty or whitespace only.
    #[error("faction is required")]
    FactionJoinKeyMissing,
    /// An ORBAT faction join key carries leading or trailing whitespace.
    #[error("faction must not have leading or trailing whitespace")]
    FactionJoinKeyPadded,
}

impl From<String> for Error {
    /// A refusal clause written by one of the block modules' field checks.
    fn from(clause: String) -> Self {
        Self::Refused(clause)
    }
}

impl From<Error> for String {
    /// The refusal as the sentence a caller shows or folds into its own message.
    fn from(error: Error) -> Self {
        error.to_string()
    }
}

/// The result of a mission model check.
pub type Result<T> = std::result::Result<T, Error>;
