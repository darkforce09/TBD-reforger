//! Dialog focus for the top command strip.

use super::*;

#[cfg(target_arch = "wasm32")]
/// Return whether the focused element belongs to the dialog node list.
pub(super) fn within(active: &Option<wasm_bindgen::JsValue>, nodes: &web_sys::NodeList) -> bool {
    let Some(active) = active else { return false };
    for i in 0..nodes.length() {
        if let Some(n) = nodes.item(i) {
            if *active == *AsRef::<wasm_bindgen::JsValue>::as_ref(&n) {
                return true;
            }
        }
    }
    false
}

/// dialog subtree (`root`) instead of walking out into the left dock. Enumerates the dialog's own
/// focusables in DOM order (✕ → version → notes → Save) and wraps at the edges: Shift+Tab off the
/// first goes to the last, Tab off the last goes to the first; a Tab that arrives with focus already
/// outside the set is pulled back to the first. Only `Tab` is acted on — Escape still bubbles to the
/// strip's window listener, and ordinary typing is untouched. wasm-only (`NodeList` /
/// `query_selector_all` are not in the native web-sys feature set); the native build takes the
/// no-op below (the trap only has meaning against a live DOM).
#[cfg(target_arch = "wasm32")]
pub(super) fn trap_tab_in_dialog(
    dialog_ref: NodeRef<leptos::html::Div>,
    ev: &web_sys::KeyboardEvent,
) {
    use wasm_bindgen::JsCast;
    if ev.key() != "Tab" {
        return;
    }
    let Some(root) = dialog_ref.get_untracked() else {
        return;
    };
    let root: &web_sys::Element = root.as_ref();
    let Ok(nodes) = root.query_selector_all(
        "button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
    ) else {
        return;
    };
    let len = nodes.length();
    if len == 0 {
        return;
    }
    let first = nodes.item(0);
    let last = nodes.item(len - 1);
    let active = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .map(wasm_bindgen::JsValue::from);
    let is = |a: &Option<wasm_bindgen::JsValue>, b: &Option<web_sys::Node>| match (a, b) {
        (Some(a), Some(b)) => *a == *AsRef::<wasm_bindgen::JsValue>::as_ref(b),
        _ => false,
    };
    let focus_node = |n: Option<web_sys::Node>| {
        if let Some(el) = n.and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    };
    if ev.shift_key() {
        if is(&active, &first) || !within(&active, &nodes) {
            ev.prevent_default();
            focus_node(last);
        }
    } else if is(&active, &last) || !within(&active, &nodes) {
        ev.prevent_default();
        focus_node(first);
    }
}

/// Native no-op — the Tab trap only has meaning against a live DOM (see the wasm variant above).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn trap_tab_in_dialog(
    _dialog_ref: NodeRef<leptos::html::Div>,
    _ev: &web_sys::KeyboardEvent,
) {
}
