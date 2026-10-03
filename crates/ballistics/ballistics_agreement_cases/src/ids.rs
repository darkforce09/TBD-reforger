//! The typed identifier of one drawn agreement case.
//!
//! **Role:** declares [`AgreementCaseId`], `<seed as 16 hex digits>_<index as 4 digits>`.
//! **Position:** held by every [`crate::AgreementCase`]; the agreement gate of the developer
//! tools and the agreement bench of the single-page app key their verdicts by it.
//! **Signals & state:** none; a plain data type.
//! **Invariants:** it serialises as the bare string it wraps, so a recorded verdict keeps its
//! bytes.

use newtype_ids::string_id;

string_id! {
    /// The identifier of one drawn case: `<seed as 16 hex digits>_<index as 4 digits>`.
    pub struct AgreementCaseId;
}
