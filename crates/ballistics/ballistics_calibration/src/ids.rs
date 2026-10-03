//! The typed identifier of one judged calibration case.
//!
//! **Role:** declares [`CalibrationCaseId`], the stable name of a case or provenance check, such
//! as `native/<shell>/<rings>/<elevation>` or `provenance/catalog_sha256`.
//! **Position:** held by every [`crate::CalibrationFailure`]; the judges of [`crate::charge_flight`],
//! [`crate::native_tables`], [`crate::wind_tables`], [`crate::oracle_samples`] and
//! [`crate::provenance`] name their cases with it.
//! **Signals & state:** none; a plain data type.
//! **Invariants:** it serialises as the bare string it wraps, so an upload report's JSON keeps its
//! bytes.

use newtype_ids::string_id;

string_id! {
    /// The stable identifier of one calibration case or provenance check.
    pub struct CalibrationCaseId;
}
