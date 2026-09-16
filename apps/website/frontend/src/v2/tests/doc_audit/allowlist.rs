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
        path: "apps/editor/arsenal/arsenal_doll.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/arsenal/arsenal_rules.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/arsenal/asset_catalog.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/arsenal/loadout.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
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
        path: "apps/editor/arsenal/panels.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/commands.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/gestures.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/gizmo_z.rs",
        reason: "The file holds inline test modules; the editor decomposition lifts those tests \
                 into sibling `tests/` files.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/mod.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/overlays.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/render_sync.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/tactical_graphics.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/canvas/viewport.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/eden_chrome.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/layout.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/mission_editor.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/mission_size.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/attributes_modal.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/audio_emitters.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/context_menu.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/dock_left.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/dock_right.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/env.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/help_modal.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/mod.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/outliner.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/outliner_drag.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/outliner_tree.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/radio_panel.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/settings_modal.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/spawn_modules.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/tasks_panel.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/toolbelt.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/top_strip.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/validation_panel.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/vehicles_panel.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/weather_timeline.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/win_conditions_card.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/panels/zones_panel.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/commands_hotkeys.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/doc_host.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/history.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/hydrate.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/mod.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/persist.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/save_status.rs",
        reason: "The file holds inline test modules and has comments naming tickets and waves; \
                 the editor decomposition lifts those tests into sibling `tests/` files and \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/session.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/tab_lock.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/state/title_prefer.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/tools/los_tool.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/tools/los_world_wasm.rs",
        reason: "The file has comments naming tickets and waves; the editor decomposition \
                 rewrites that prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/tools/ruler_tool.rs",
        reason: "The file is over the line limit, holds inline test modules and has comments \
                 naming tickets and waves; the editor decomposition splits it into per-surface \
                 modules, lifts those tests into sibling `tests/` files and rewrites that prose \
                 in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/tools/select_tool.rs",
        reason: "The file is over the line limit and has comments naming tickets and waves; the \
                 editor decomposition splits it into per-surface modules and rewrites that \
                 prose in the present tense.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
    GrandfatherRow {
        path: "apps/editor/world_layer_prefs.rs",
        reason: "The file holds inline test modules; the editor decomposition lifts those tests \
                 into sibling `tests/` files.",
        expires: EDITOR_DECOMPOSITION_DUE,
    },
];
