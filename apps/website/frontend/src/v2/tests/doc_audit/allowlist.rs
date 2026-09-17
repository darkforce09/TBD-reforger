//! Production files exempted from the v2 documentation audit.
//!
//! **Role:** names any files temporarily exempt from size, inline-test-module, and
//! ticket/wave-comment rules.
//! **Position:** read only by the documentation audit in this directory.
//! **Signals & state:** none; the table is a compile-time constant.
//! **Invariants:** every row names an existing file and carries a nonempty reason and
//! expiry date. An empty table means the tree meets the whole standard unaided.

use super::GrandfatherRow;

/// Files currently exempt from audit rules two, four, and five.
pub(super) const ROWS: &[GrandfatherRow] = &[];
