//! The dated grandfather rows the v2 documentation audit honours.
//!
//! **Role:** names each production file that is carried past the audit's size, inline-test-module
//! and ticket/wave rules, why it is carried, and the last day the exemption holds.
//! **Position:** read only by the audit in this directory; nothing else consumes it.
//! **Signals & state:** none. The table is a compile-time constant.
//! **Invariants:** every row names a file the audit's walk sees, carries a non-empty reason, and
//! expires on a real `YYYY-MM-DD` that has not passed. A row whose file is split, moved or
//! deleted is updated or removed in the same change, because an orphaned row fails the audit.
//! A row is a debt with a due date, never a permanent exclusion.

use super::GrandfatherRow;

/// Every file currently exempt from audit rules two, four and five. An empty table means the
/// tree meets the whole standard unaided.
pub(super) const ROWS: &[GrandfatherRow] = &[];
