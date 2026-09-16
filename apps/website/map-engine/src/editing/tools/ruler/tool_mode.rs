//! Role: which tool owns the left button, and whether a press is a point capture.
//! Position: `editing/tools/ruler` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: one enum decides what a click MEANS, so the toolbar and the pointer handlers can never disagree about the active tool.

/// `Ruler` re-purposes the LMB for measuring (T-642). `LoS` (T-643, wave 109) re-purposes it for a
/// point-to-point line-of-sight ray — `Ruler`'s neighbour button (`visibility`). Both `Ruler` and
/// `LoS` are "click points on the map" tools that share the SAME `LG::Ruler` left-gesture arm (a
/// sub-threshold LMB click commits one world point); which tool is live decides what a committed
/// click MEANS — a ruler vertex vs a LoS observer/target. So this enum is the "mode field on the
/// ruler arm" the ticket calls for: `LeftGesture` stays a two-variant `select_tool` type (not
/// touched by T-643), and the commit site branches on `tool_mode` read here.
///
/// This is a shared, native-testable enum (the `eden_toolbelt` buttons read it via a signal; the
/// `mission_editor` pointer handlers branch on it). Keeping it here — not in the wasm-only
/// `select_tool` — is what lets `cargo test -p website-frontend` prove the arbitration below and in
/// `los_tool`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorTool {
    #[default]
    Select,
    Ruler,
    /// T-643 — Line of Sight: click observer, click target → clear/blocked + terrain profile.
    LoS,
}

impl EditorTool {
    #[must_use]
    pub fn is_ruler(self) -> bool {
        matches!(self, EditorTool::Ruler)
    }

    /// T-643 — true when Line of Sight is the active tool.
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
/// The whole tool-mode arbitration in one predicate — and the answer to the ticket's binding
/// constraints (the wave-106 T-723 findings):
///   * **(c) button 0 only** — a captured point is a LEFT click; middle/right stay pan / context
///     menu. `button != 0` ⇒ never a capture press (so the host's MMB-pan and RMB-menu are
///     untouched).
///   * the tool must CAPTURE POINTS — `Ruler` (T-642) or `LoS` (T-643). Both ride the SAME
///     `LG::Ruler` arm; the pointerup commit site branches on `tool_mode` (`is_ruler()` /
///     `is_los()`) to route the point. Under `Select` this is always `false` and the existing
///     Pending→Move|Marquee path is entirely unchanged.
///
/// The name is kept `should_begin_ruler` (T-642 pins + the `select_tool` docs reference it) even
/// though it now also opens the LoS capture: LoS deliberately REUSES the ruler's `LG` arm rather
/// than adding a third `LeftGesture` variant to the un-owned `select_tool` — the "mode field on the
/// ruler arm" the ticket sanctions.
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

// ── The chain state machine (Decision 3 lives here) ─────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/tool_mode.rs"]
mod tests;
