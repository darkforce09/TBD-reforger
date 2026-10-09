//! Role: which tool owns the left button, and whether a press is a point capture.
//! Position: `ruler` in `map_editing_tools`.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: one enum decides what a click MEANS, so the toolbar and the pointer handlers can never disagree about the active tool.

/// The tool that owns the left button.
///
/// `Ruler` re-purposes the left button for measuring and `LoS` for a point-to-point line-of-sight
/// ray. Both are "click points on the map" tools that share the same `LeftGesture::Ruler` arm (a
/// sub-threshold left click commits one world point); which tool is live decides what a committed
/// click means — a ruler vertex or a line-of-sight observer or target. The commit site branches on
/// this enum, so the gesture enum needs no variant per measuring tool.
///
/// A shared, native-testable enum: the toolbelt buttons read it through a signal and the Mission
/// Creator's pointer handlers branch on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorTool {
    /// The left button selects, moves and marquees.
    #[default]
    Select,
    /// The left button commits ruler vertices.
    Ruler,
    /// Line of Sight: click observer, click target → clear/blocked + terrain profile.
    LoS,
}

impl EditorTool {
    /// Whether the ruler owns the left button.
    #[must_use]
    pub fn is_ruler(self) -> bool {
        matches!(self, EditorTool::Ruler)
    }

    /// True when Line of Sight is the active tool.
    #[must_use]
    pub fn is_los(self) -> bool {
        matches!(self, EditorTool::LoS)
    }

    /// True when the tool captures map CLICKS as points (Ruler or LoS) rather than driving the
    /// Select pick/marquee/move machine. This is what the LMB pointerdown reads to decide whether to
    /// open `LG::Ruler` (the shared point-capture gesture) instead of `LG::Pending`; the commit site
    /// then branches on `is_ruler()` / `is_los()` to route the point. Select → `false` (the whole
    /// Select machine is byte-for-byte unchanged).
    #[must_use]
    pub fn captures_points(self) -> bool {
        self.is_ruler() || self.is_los()
    }
}

/// Should an LMB `pointerdown` open the shared POINT-CAPTURE gesture (`LeftGesture::Ruler`) rather
/// than the Select machine's `Pending`?
///
/// The whole tool-mode arbitration in one predicate:
///   * **(c) button 0 only** — a captured point is a LEFT click; middle/right stay pan / context
///     menu. `button != 0` ⇒ never a capture press (so the host's MMB-pan and RMB-menu are
///     untouched).
///   * the tool must CAPTURE POINTS — `Ruler` or `LoS`. Both ride the SAME
///     `LG::Ruler` arm; the pointerup commit site branches on `tool_mode` (`is_ruler()` /
///     `is_los()`) to route the point. Under `Select` this is always `false` and the existing
///     Pending→Move|Marquee path is entirely unchanged.
///
/// The name is `should_begin_ruler` although it also opens the line-of-sight capture: that tool
/// reuses the ruler's gesture arm rather than adding a `LeftGesture` variant of its own.
///
/// The host uses this at `pointerdown` to choose `LeftGesture::Ruler` vs `LeftGesture::Pending`, and
/// the arm it opens is a SEPARATE `LG` arm that never falls into the armed-placement pointerup
/// branch — constraint **(a)** (that branch is gated on a palette place, which a capture click never
/// arms) — and whose pointerdown-written gesture is always taken/cleared by the pointermove/up/
/// cancel arms — constraint **(b)**.
#[must_use]
pub fn should_begin_ruler(tool: EditorTool, button: i16) -> bool {
    tool.captures_points() && button == 0
}
