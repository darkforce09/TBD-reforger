//! The ids an NDJSON debug row carries.
//!
//! **Role:** [`RunId`] (the `runId` of every row of one run) and [`HypothesisId`] (the
//! `hypothesisId` of one row, `H1` to `H6` for the direct-join block).
//! **Position:** taken by [`crate::debug::probes`] and [`crate::debug::direct_join`]; built by the
//! dispatch from the command line's text.
//! **Signals & state:** none; plain values.
//! **Invariants:** each id is written into the row as its bare string, so a row reads exactly as
//! the text the command line gave.

newtype_ids::string_id! {
    /// The run an NDJSON debug row belongs to (`runId`); `user-repro` when `direct-join` is given
    /// none.
    pub struct RunId;
}

newtype_ids::string_id! {
    /// The hypothesis an NDJSON debug row records (`hypothesisId`), such as `H1`.
    pub struct HypothesisId;
}
