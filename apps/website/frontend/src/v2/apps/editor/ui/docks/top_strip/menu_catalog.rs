//! Menu catalog for the top command strip.

use super::*;

/// Top-strip menu labels and their command entries.
pub(super) const MENUS: [(&str, &[MenuItem]); 6] = [
    (
        "File",
        &[
            MenuItem {
                label: "Save Version…",
                action: Some(MenuAction::Save),
            },
            MenuItem {
                label: "Export JSON",
                action: Some(MenuAction::Export),
            },
            MenuItem {
                label: "Export Compiled Mission",
                action: Some(MenuAction::ExportCompiled),
            },
        ],
    ),
    (
        "Edit",
        &[
            MenuItem {
                label: "Undo (Ctrl+Z)",
                action: Some(MenuAction::Undo),
            },
            MenuItem {
                label: "Redo (Ctrl+Shift+Z)",
                action: Some(MenuAction::Redo),
            },
            MenuItem {
                label: "Select All on Screen (Ctrl+A)",
                action: Some(MenuAction::SelectAll),
            },
            MenuItem {
                label: "Widget: No Widget (1)",
                action: Some(MenuAction::SetWidget(1)),
            },
            MenuItem {
                label: "Widget: Translation (2)",
                action: Some(MenuAction::SetWidget(2)),
            },
            MenuItem {
                label: "Widget: Rotation (3)",
                action: Some(MenuAction::SetWidget(3)),
            },
            MenuItem {
                label: "Toggle Snap Grid (G)",
                action: Some(MenuAction::ToggleSnap),
            },
            MenuItem {
                label: "Snap Step — Decrease ([)",
                action: Some(MenuAction::SnapStep(-1)),
            },
            MenuItem {
                label: "Snap Step — Increase (])",
                action: Some(MenuAction::SnapStep(1)),
            },
        ],
    ),
    ("Arrange", &ARRANGE_ITEMS),
    (
        "Mission",
        &[
            MenuItem {
                label: "Mission Settings…",
                action: Some(MenuAction::Settings),
            },
            MenuItem {
                label: "Briefing & Thumbnail…",
                action: Some(MenuAction::Settings),
            },
        ],
    ),
    (
        "Environment",
        &[MenuItem {
            label: "Time & Weather…",
            action: Some(MenuAction::Settings),
        }],
    ),
    (
        "Help",
        &[MenuItem {
            label: "Keyboard Shortcuts (Controls Hint)",
            action: Some(MenuAction::ControlsHint),
        }],
    ),
];

/// live selection off `editor_ops` under wasm; the native view shell has no doc/selection, so it
/// reports 0 (the placement rows render disabled there — the menu is a wasm-only affordance anyway).
#[must_use]
pub(super) fn selection_count() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        website_map_engine::editing::host::selection_len()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}
