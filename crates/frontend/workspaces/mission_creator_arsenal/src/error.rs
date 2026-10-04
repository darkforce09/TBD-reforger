//! The crate's error: why a loadout is refused before it reaches a file or the mission document.
//!
//! **Role:** the one failure type of the crate's fallible public functions: the export gate
//! (`loadout::try_export`), the import gate (`loadout::try_import`), the buffered Apply plan
//! (`loadout::plan_apply`) and its document write
//! (`loadout_commands::apply_loadout_buffer_to_selection`).
//! **Position:** built where a gate refuses a loadout: the JSON parse and the shipped export schema
//! of an import, the capacity rules of an export, the compatibility and capacity rules of an import
//! or a buffered Apply. Read by the Arsenal tab, which lists one line per refused row under the
//! verdict, and by the tests that pin which rows a gate refuses.
//! **Signals & state:** none; plain data.
//! **Invariants:** a refusal carries every refused row in the order the gate reported it, each
//! with its pick key and its operator-facing sentence; the `Display` is one
//! `loadout::refusal_line` per row, joined with a newline; no caller branches on the variant.

use mission_creator_state::arsenal_rules::RowError;

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why a loadout is refused before it reaches a file or the mission document.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum Error {
    /// The loadout breaks one or more rules: one refusal per row at fault, nothing applied or
    /// written.
    #[error("{}", refusal_lines(.0))]
    LoadoutRefused(Vec<RowError>),
}

impl Error {
    /// The refused rows, in report order.
    #[must_use]
    pub fn refusals(&self) -> &[RowError] {
        match self {
            Self::LoadoutRefused(refusals) => refusals,
        }
    }

    /// The refused rows, in report order, by value.
    #[must_use]
    pub fn into_refusals(self) -> Vec<RowError> {
        match self {
            Self::LoadoutRefused(refusals) => refusals,
        }
    }
}

/// One `refusal_line` per refused row, joined with a newline.
fn refusal_lines(refusals: &[RowError]) -> String {
    refusals
        .iter()
        .map(crate::loadout::refusal_line)
        .collect::<Vec<_>>()
        .join("\n")
}
