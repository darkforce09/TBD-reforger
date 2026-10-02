//! Menu geometry.

use super::*;

/// Returns indexes of enabled, nonseparator menu rows.
#[must_use]
pub fn selectable_indices(entries: &[MenuEntry]) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.item.is_some() && e.enabled)
        .map(|(i, _)| i)
        .collect()
}

/// Moves keyboard highlight to the next selectable row.
#[must_use]
pub fn step_highlight(entries: &[MenuEntry], cur: Option<usize>, dir: i32) -> Option<usize> {
    let sel = selectable_indices(entries);
    if sel.is_empty() {
        return None;
    }
    let pos = cur.and_then(|c| sel.iter().position(|&i| i == c));
    let next = match (pos, dir) {
        (None, d) if d > 0 => 0,
        (None, _) => sel.len() - 1,
        (Some(p), d) if d > 0 => (p + 1).min(sel.len() - 1),
        (Some(p), _) => p.saturating_sub(1),
    };
    Some(sel[next])
}

#[cfg(any(target_arch = "wasm32", test))]
/// Positions a menu axis within the visible viewport.
pub(super) fn menu_axis_position(anchor: f64, extent: f64, viewport: f64) -> f64 {
    anchor.clamp(8.0, (viewport - extent - 8.0).max(8.0))
}

#[cfg(any(target_arch = "wasm32", test))]
/// Computes the scroll offset that reveals a keyboard-selected row.
pub(super) fn menu_scroll_top(scroll: f64, height: f64, row_top: f64, row_height: f64) -> f64 {
    if row_top < scroll {
        row_top.max(0.0)
    } else if row_top + row_height > scroll + height {
        (row_top + row_height - height).max(0.0)
    } else {
        scroll
    }
}

/// Bounds and clipping constraints applied to the measured menu.
pub(super) const MENU_BOUNDS: &str = "box-sizing:border-box;min-width:min(15rem,calc(100vw - 16px));max-width:min(20rem,calc(100vw - 16px));max-height:calc(100dvh - 16px);";

#[cfg(target_arch = "wasm32")]
/// Scrolls the highlighted row into the visible menu panel.
pub(super) fn reveal_menu_row(panel: &web_sys::HtmlDivElement, highlight: Option<usize>) {
    let Some(idx) = highlight else { return };
    let Ok(Some(row)) = panel.query_selector(&format!("[data-context-row='{idx}']")) else {
        return;
    };
    let scroll = f64::from(panel.scroll_top());
    let row_rect = row.get_bounding_client_rect();
    let row_top =
        row_rect.top() - panel.get_bounding_client_rect().top() - f64::from(panel.client_top())
            + scroll;
    let next = menu_scroll_top(
        scroll,
        f64::from(panel.client_height()),
        row_top,
        row_rect.height(),
    );
    panel.set_scroll_top(next.ceil() as i32);
}

#[cfg(target_arch = "wasm32")]
/// Places the measured menu panel inside the viewport.
pub(super) fn place_context_menu(panel: &web_sys::HtmlDivElement, state: &MenuState) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(width) = window.inner_width().ok().and_then(|v| v.as_f64()) else {
        return;
    };
    let Some(height) = window.inner_height().ok().and_then(|v| v.as_f64()) else {
        return;
    };
    let _ = panel.set_attribute("style", &format!("{MENU_BOUNDS}left:8px;top:8px"));
    let rect = panel.get_bounding_client_rect();
    let x = menu_axis_position(state.x, rect.width(), width);
    let y = menu_axis_position(state.y, rect.height(), height);
    let _ = panel.set_attribute("style", &format!("{MENU_BOUNDS}left:{x}px;top:{y}px"));
}
