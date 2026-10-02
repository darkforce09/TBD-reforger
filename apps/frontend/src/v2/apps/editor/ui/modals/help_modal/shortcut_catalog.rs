//! Editor keyboard shortcuts and display groups.

use super::*;

/// One documented editor shortcut.
pub struct Shortcut {
    pub codes: &'static [&'static str],
    pub chord: &'static str,
    pub action: &'static str,
    pub group: &'static str,
}

/// Shortcut group headings in render order.
pub const GROUPS: [&str; 7] = [
    "Selection",
    "View",
    "Transform & snapping",
    "Arrange",
    "History",
    "Tools",
    "Context menu",
];

/// Editor shortcuts bound to live key handlers.
pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        codes: &["KeyA"],
        chord: "Ctrl/Cmd + A",
        action: "Select all in view",
        group: "Selection",
    },
    Shortcut {
        codes: &["KeyC"],
        chord: "Ctrl/Cmd + C",
        action: "Copy the selection",
        group: "Selection",
    },
    Shortcut {
        codes: &["KeyX"],
        chord: "Ctrl/Cmd + X",
        action: "Cut the selection (copy, then remove)",
        group: "Selection",
    },
    Shortcut {
        codes: &["KeyV"],
        chord: "Ctrl/Cmd + V",
        action: "Paste at the cursor",
        group: "Selection",
    },
    Shortcut {
        codes: &["KeyV"],
        chord: "Ctrl/Cmd + Shift + V",
        action: "Paste at the source position instead of the cursor",
        group: "Selection",
    },
    Shortcut {
        codes: &["Delete"],
        chord: "Delete",
        action: "Remove the selection",
        group: "Selection",
    },
    Shortcut {
        codes: &["Space"],
        chord: "Space",
        action: "Centre the camera on the selection",
        group: "Selection",
    },
    Shortcut {
        codes: &["Backspace"],
        chord: "Backspace",
        action: "Hide / show the whole interface (this card included)",
        group: "View",
    },
    Shortcut {
        codes: &["KeyE"],
        chord: "E",
        action: "Collapse / expand the Entity List (left dock)",
        group: "View",
    },
    Shortcut {
        codes: &["KeyR"],
        chord: "R",
        action: "Collapse / expand the Asset Browser (right dock)",
        group: "View",
    },
    Shortcut {
        codes: &["KeyD"],
        chord: "Ctrl/Cmd + Alt + D",
        action: "Toggle the telemetry HUD in the status bar",
        group: "View",
    },
    Shortcut {
        codes: &["KeyG"],
        chord: "G",
        action: "Toggle the snap grid",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["BracketLeft", "BracketRight"],
        chord: "[  /  ]",
        action: "Decrease / increase the snap step of the active widget",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["Digit1"],
        chord: "1",
        action: "No widget (bare drag still moves the selection)",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["Digit2"],
        chord: "2",
        action: "Translate widget",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["Digit3"],
        chord: "3",
        action: "Rotate widget (drag the ring to rotate the selection)",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["KeyL", "KeyR"],
        chord: "Alt + L  /  Alt + R",
        action: "Align the selection to its left / right edge (needs 2+ selected)",
        group: "Arrange",
    },
    Shortcut {
        codes: &["KeyT", "KeyB"],
        chord: "Alt + T  /  Alt + B",
        action: "Align the selection to its top / bottom edge (needs 2+ selected)",
        group: "Arrange",
    },
    Shortcut {
        codes: &["KeyH", "KeyV"],
        chord: "Alt + H  /  Alt + V",
        action: "Space the selection equally, horizontally / vertically (needs 2+ selected)",
        group: "Arrange",
    },
    Shortcut {
        codes: &["KeyZ"],
        chord: "Ctrl/Cmd + Z",
        action: "Undo",
        group: "History",
    },
    Shortcut {
        codes: &["KeyY"],
        chord: "Ctrl/Cmd + Y  or  Ctrl/Cmd + Shift + Z",
        action: "Redo",
        group: "History",
    },
    Shortcut {
        codes: &["Escape"],
        chord: "Esc",
        action: "Dismiss whatever is up: a measurement, an open menu or dropdown, the Save dialog, \
                 the Attributes modal, the asset picker, the comment editor, the connections panel, \
                 a settings dialog, the context menu, the Faction or ORBAT Manager — or this card",
        group: "Tools",
    },
    Shortcut {
        codes: &["ArrowUp", "ArrowDown"],
        chord: "↑  /  ↓",
        action: "Move the highlight while the context menu is open",
        group: "Context menu",
    },
    Shortcut {
        codes: &["Enter"],
        chord: "Enter",
        action: "Run the highlighted row (or expand it, if it opens a submenu)",
        group: "Context menu",
    },
];
