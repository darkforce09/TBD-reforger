//! Which right-hand columns fit the window.
//!
//! **Role:** `RightPane` and `right_pane`, the choice between the detail column, the document
//! viewer, both or neither, with the detail width and the two-column threshold.
//! **Position:** called by the desktop application each frame before it paints the right region.
//! **Signals & state:** none; pure functions over the window width and the selection.
//! **Invariants:** closing the viewer never changes the selection; below the two-column threshold
//! the viewer shows alone.

/// The detail column's width, in points.
pub const DETAIL_W: f32 = 420.0;

/// Below this window width the viewer occupies the right region alone; the detail panel
/// returns alongside it when both columns fit without clipping.
pub const TWO_COLUMN_MIN_WINDOW_W: f32 = 1100.0;

// ---- in-app markdown viewer pane ----

/// Which right-hand pane(s) render this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPane {
    /// Viewer column alone — no selection (a reload may drop it under an open
    /// document), or the narrow-window degrade.
    Viewer,
    /// The detail column alone, for the ticket at this corpus index.
    Detail(usize),
    /// both columns — the detail panel with the viewer to its right.
    Both(usize),
    /// Neither column.
    None,
}

/// The right region contains the detail column and a viewer column to its right.
/// Closing the viewer leaves selection intact. The viewer is added first so it
/// occupies the right edge; narrow windows show the viewer alone.
pub fn right_pane(viewer_open: bool, selected: Option<usize>, window_w: f32) -> RightPane {
    match (viewer_open, selected) {
        (false, None) => RightPane::None,
        (false, Some(index)) => RightPane::Detail(index),
        (true, None) => RightPane::Viewer,
        (true, Some(_)) if window_w < TWO_COLUMN_MIN_WINDOW_W => RightPane::Viewer,
        (true, Some(index)) => RightPane::Both(index),
    }
}

#[cfg(test)]
#[path = "tests/window_layout.rs"]
mod tests;
