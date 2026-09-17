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

/// The last day the diagnostics-bench rows exempt anything. The decomposition that splits the
/// benches and lifts their tests runs before it, on the same schedule as the editor's.
const DIAGNOSTICS_DECOMPOSITION_DUE: &str = "2026-12-31";

/// Every file currently exempt from audit rules two, four and five. An empty table means the
/// tree meets the whole standard unaided.
pub(super) const ROWS: &[GrandfatherRow] = &[
    GrandfatherRow {
        path: "apps/debug/building_interior.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 diagnostics decomposition splits it into per-lane modules and rewrites that \
                 prose in the present tense.",
        expires: DIAGNOSTICS_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/debug/building_viewer.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the diagnostics decomposition splits it into a pure \
                 geometry module and a wasm host, lifts those tests into sibling `tests/` files \
                 and rewrites that prose in the present tense.",
        expires: DIAGNOSTICS_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/debug/world_los.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 diagnostics decomposition splits its wasm host off from its view and rewrites \
                 that prose in the present tense.",
        expires: DIAGNOSTICS_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/debug/world_los_scene.rs",
        reason: "The file holds an inline test module and has comments naming tickets and waves; \
                 the diagnostics decomposition lifts those tests into a sibling `tests/` file and \
                 rewrites that prose in the present tense.",
        expires: DIAGNOSTICS_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/arsenal/doll.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/arsenal/mod.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/bridge/document_host/doc_host.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/bridge/document_host/history.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/bridge/gizmo_z.rs",
        reason: "The file holds inline test modules; the editor decomposition lifts those tests \
                 into sibling `tests/` files.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/bridge/overlays.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/bridge/tactical_graphics.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/bridge/viewport.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/input/pointer_gestures.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/input/tools/los_tool.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/input/tools/los_world_wasm.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/input/tools/ruler_tool.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/input/tools/select_tool.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/input/window_keydown.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
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
    GrandfatherRow {
        path: "apps/editor/ui/arsenal/panels.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/audio_emitters.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/env.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/radio_panel.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/spawn_modules.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/tasks_panel.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/vehicles_panel.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/weather_timeline.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/inspector/win_conditions_card.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/outliner/drag.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/outliner/outliner.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/ui/outliner/tree.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
];
