//! Clipboard browser commands.
use super::*;
use crate::v2::core::utils::clipboard::write_clipboard;
/* ───────────────  the browser half of the clipboard exporters ─────────────── */

/// The author-facing refusal when a clipboard exporter runs with nothing selected. A copy that
/// quietly did nothing is indistinguishable from a copy that worked until the paste lands empty.
pub(super) const NOTHING_SELECTED: &str =
    "Nothing is selected — select an entity on the map first.";

/// The live selection ids.
///
/// **Why through the `window.__editorSelection` bridge rather than a Rust call.** The selection
/// is app-side state held in `select_tool`'s leaked `SelectionHandle` and mirrored in
/// the installed `EDITOR_CONTEXT`; neither exposes a Rust ids accessor (`website_map_engine::editing::host::selection_len`
/// returns only the count, and `attrs_multi_ids` needs an anchor id and refuses below two). The
/// one exported reader is `__editorSelection.ids()`, which `select_tool::register_editor_selection`
/// installs over the same handle  so this reads the real selection, not a copy that can drift.
/// An `editor_ops::selection_ids()` would be the better seam and is reported as residue.
///
/// Every failure along the way yields an EMPTY selection, which the callers turn into the
/// [`NOTHING_SELECTED`] refusal  never into a copy of something else.
pub(super) fn selected_ids() -> Vec<String> {
    let Some(win) = web_sys::window() else {
        return Vec::new();
    };
    let Ok(bridge) = js_sys::Reflect::get(&win, &JsValue::from_str("__editorSelection")) else {
        return Vec::new();
    };
    let Ok(f) = js_sys::Reflect::get(&bridge, &JsValue::from_str("ids")) else {
        return Vec::new();
    };
    let Ok(f) = f.dyn_into::<js_sys::Function>() else {
        return Vec::new();
    };
    let Ok(raw) = f.call0(&bridge) else {
        return Vec::new();
    };
    raw.as_string()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

/// The live selection resolved against the hosted document  the input every exporter shares.
pub(super) fn selection_entities() -> Vec<SelectedEntity> {
    let ids = selected_ids();
    if ids.is_empty() {
        return Vec::new();
    }
    EDITOR_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        resolve_selected_entities(&core.slots_json(), &core.small_maps_json(), &ids)
    })
}

/// ** exporter 1  copy the selection's grid position.**
///
/// `#[allow(dead_code)]`: this verb has no UI entry point yet. The menu bar (`eden_top_strip.rs`,
/// where `export_compiled_now`'s button lives) and the context menu (`context_menu.rs`) are both
/// separate views; the three exporters are commands callable by either surface. They are harness-drivable
/// today through `__editorCommands.clipboard_grid_json()` and its two peers.
#[allow(dead_code)]
pub fn copy_grid_position_now(toasts: crate::v2::core::ui::toast::Toasts) {
    let entities = selection_entities();
    if entities.is_empty() {
        toasts.error(NOTHING_SELECTED);
        return;
    }
    let text = super::super::grid_position_text(&entities);
    let ok = if entities.len() == 1 {
        format!("Copied the grid reference {text}.")
    } else {
        format!(
            "Copied {}.",
            super::super::count_noun(entities.len(), "grid reference", "grid references")
        )
    };
    write_clipboard(text, ok, toasts);
}

/// ** exporter 2  copy the selection's classnames.** Same missing-menu-entry residue note
/// as [`copy_grid_position_now`].
///
/// A selection whose every entity is classname-less copies NOTHING and says so: putting an empty
/// string on the clipboard while reporting success is the same silent-failure shape the awaited
/// promise exists to prevent.
#[allow(dead_code)]
pub fn copy_classnames_now(toasts: crate::v2::core::ui::toast::Toasts) {
    let entities = selection_entities();
    if entities.is_empty() {
        toasts.error(NOTHING_SELECTED);
        return;
    }
    let (text, skipped) = super::super::classnames_text(&entities);
    if text.is_empty() {
        toasts.error("Nothing in the selection carries a classname — nothing was copied.");
        return;
    }
    let copied = entities.len() - skipped;
    let ok = if skipped == 0 {
        format!(
            "Copied {}.",
            super::super::count_noun(copied, "classname", "classnames")
        )
    } else {
        format!(
            "Copied {}, skipping {} with no classname.",
            super::super::count_noun(copied, "classname", "classnames"),
            super::super::count_noun(skipped, "entity", "entities")
        )
    };
    write_clipboard(text, ok, toasts);
}

/// ** exporter 3  copy a human-readable digest of the selection.** Same missing-menu-entry
/// residue note as [`copy_grid_position_now`].
#[allow(dead_code)]
pub fn copy_selection_summary_now(toasts: crate::v2::core::ui::toast::Toasts) {
    let entities = selection_entities();
    if entities.is_empty() {
        toasts.error(NOTHING_SELECTED);
        return;
    }
    let text = super::super::selection_summary_text(&entities);
    let ok = format!(
        "Copied a summary of {}.",
        super::super::count_noun(entities.len(), "entity", "entities")
    );
    write_clipboard(text, ok, toasts);
}

/// what a clipboard exporter WOULD put on the clipboard, for the harness.
///
/// `{"text":…,"count":n,"skipped":k}` on success, `{"error":…}` on a refusal  the two are
/// distinguishable by shape, the `compiled_diagnostics_json` precedent. This deliberately does
/// NOT touch the clipboard: a headless gate has no clipboard permission, and a reader that had
/// to grant one would test the browser rather than the exporter. The clipboard write itself is
/// `crate::v2::core::utils::clipboard::write_clipboard`, and its contract (await, then report) is
/// prose the author can check against the toast.
pub(super) fn export_preview_json(kind: &str) -> String {
    let entities = selection_entities();
    if entities.is_empty() {
        return serde_json::json!({ "error": NOTHING_SELECTED }).to_string();
    }
    let (text, skipped) = match kind {
        "classnames" => super::super::classnames_text(&entities),
        "summary" => (super::super::selection_summary_text(&entities), 0),
        _ => (super::super::grid_position_text(&entities), 0),
    };
    serde_json::json!({
        "text": text,
        "count": entities.len(),
        "skipped": skipped,
    })
    .to_string()
}
