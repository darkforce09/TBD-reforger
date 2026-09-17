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

/// The last day the rows below exempt anything. The editor decomposition that clears them runs
/// before it, so this table drains on a date rather than on goodwill.
const EDITOR_DECOMPOSITION_DUE: &str = "2026-12-31";

/// Every file currently exempt from audit rules two, four and five. An empty table means the
/// tree meets the whole standard unaided.
pub(super) const ROWS: &[GrandfatherRow] = &[
    GrandfatherRow {
        path: "apps/editor/shell/document_commands.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/eden_chrome.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/hydrate.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/layout.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/mission_size.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/persist.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/save_status.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/session.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/tab_lock.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/title_prefer.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/shell/world_layer_prefs.rs",
        reason: "The file holds inline test modules; the editor decomposition lifts those tests \
                 into sibling `tests/` files.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
];
