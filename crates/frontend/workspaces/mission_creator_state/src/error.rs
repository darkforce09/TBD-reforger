//! The crate's error: why a loadout document is refused against the shipped export schema.
//!
//! **Role:** the one failure type of the crate's fallible public function,
//! [`crate::arsenal_rules::validate_against_loadout_export_schema`].
//! **Position:** built where an imported loadout document is checked against the embedded
//! `contracts/definitions/loadout-export.schema.json`; read by the Arsenal's loadout import, which
//! shows each refusal as one row of the import's fault list.
//! **Signals & state:** none; plain data.
//! **Invariants:** every refusal is the operator-facing sentence byte for byte, in the order the
//! checker reports them; the `Display` joins them with a newline; no caller branches on the
//! variant.

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why a loadout document is refused against the shipped export schema.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The document breaks the schema, or the shipped schema holds a rule the checker cannot
    /// evaluate: one sentence per fault, capped with a closing `…and <n> more schema fault(s).`
    #[error("{}", .0.join("\n"))]
    LoadoutExportSchemaRefusals(Vec<String>),
}

impl Error {
    /// The refusal sentences, in report order.
    #[must_use]
    pub fn into_messages(self) -> Vec<String> {
        match self {
            Self::LoadoutExportSchemaRefusals(messages) => messages,
        }
    }
}
