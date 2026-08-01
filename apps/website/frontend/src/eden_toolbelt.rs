//! T-661 — the Bottom Toolbelt, split from `eden_chrome.rs`.
//!
//! Select (active) + Ruler/LoS disabled stubs, then the mono CUR X/Y/Z + SEL/OBJ/SZ readout. Not
//! cfg-gated: the native view shell renders it too (the doc-reading `sel_xyz` branch is
//! `#[cfg(target_arch = "wasm32")]` inside the memo).
#![allow(dead_code)]
use leptos::prelude::*;

use crate::ui::MaterialIcon;

// ── Toolbelt class recipes (React `overlay.ts`) ────────────────────────────────────────────────────

/// `cn(overlayPanel, 'flex items-center gap-1 px-1.5 py-1.5')`.
const TOOLBELT: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex items-center gap-1 px-1.5 py-1.5";

/// A toolbelt tool button — active (Select) vs disabled stub (Ruler / LoS).
const TOOL_ACTIVE: &str =
    "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors bg-primary/20 text-primary";
const TOOL_DISABLED: &str = "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors text-on-surface-variant opacity-30 hover:bg-transparent";

/// Format a cursor axis for the mono readout. React `BottomToolbelt.fmtCoord`:
/// `n.toFixed(3).padStart(9, ' ')`, and the off-map cell is 7 spaces + an em dash. HTML collapses
/// the leading runs in both engines — `tabular-nums` does the real aligning — so this mirrors the
/// oracle rather than "fixing" it.
fn fmt_coord(v: Option<f64>) -> String {
    match v {
        Some(n) => format!("{n:>9.3}"),
        None => "       —".to_string(),
    }
}

/// Bottom Toolbelt — Select (active) + Ruler/LoS disabled stubs, then the mono CUR X/Y/Z +
/// SEL/OBJ readout.
///
/// T-172 B2/B9: Z is DEM-fed (em-dash until the grid publishes / off-coverage), and with exactly
/// one slot selected the readout swaps CUR→SEL and shows that slot's x/y/z (React parity). The
/// per-axis `title="Cursor …"` handles stay constant — they are the frozen cur-smoke's DOM hooks.
#[component]
pub fn BottomToolbelt(
    /// Cursor world position + DEM z, `None` when the pointer is off the map (em-dash cells).
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    /// Live selection mirror — drives the CUR↔SEL swap.
    selected_ids: RwSignal<Vec<String>>,
    /// T-172 B9 — debounced compiled-payload estimate (None → `—`).
    #[prop(optional)]
    sz_bytes: Option<RwSignal<Option<usize>>>,
) -> impl IntoView {
    // Exactly-one-selected → that slot's x/y/z from the doc. Recomputes on selection change AND
    // on the post-mutation selected_ids re-set (drag commit), so it never shows a stale position.
    // (`editor_ops` is wasm-only; the native view shell always renders CUR.)
    let sel_xyz = Memo::new(move |_| -> Option<(f64, f64, f64)> {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            #[cfg(target_arch = "wasm32")]
            {
                return crate::editor_ops::read_attrs(&ids[0]).map(|a| (a.x, a.y, a.z));
            }
        }
        let _ = ids;
        None
    });
    let axis_val = move |i: usize| match sel_xyz.get() {
        Some((x, y, z)) => fmt_coord(Some([x, y, z][i])),
        None => fmt_coord(cursor.get().and_then(|c| match i {
            0 => Some(c.0),
            1 => Some(c.1),
            _ => c.2,
        })),
    };
    view! {
        <div class=TOOLBELT>
            <button type="button" class=TOOL_ACTIVE aria-pressed="true" title="Select">
                <MaterialIcon name="arrow_selector_tool" class="block text-base" />
                <span class="hidden sm:inline">"Select"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Ruler (soon)">
                <MaterialIcon name="straighten" class="block text-base" />
                <span class="hidden sm:inline">"Ruler"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Line of sight (soon)">
                <MaterialIcon name="visibility" class="block text-base" />
                <span class="hidden sm:inline">"LoS"</span>
            </button>
            <span class="mx-1 h-5 w-px bg-white/10"></span>
            <div class="flex items-center gap-2 px-1 font-mono text-code-md text-on-surface-variant">
                <span class="text-outline" title="Cursor">
                    {move || if sel_xyz.get().is_some() { "SEL" } else { "CUR" }}
                </span>
                // T-159.22 — `title` (not `aria-label`): these are roleless `<span>`s, where an
                // `aria-label` is ignored by AT and would be a fake a11y name. `title` is a real
                // tooltip AND the CUR gate's DOM handle, matching the `title="Cursor"` idiom above.
                <span title="Cursor X">
                    "X"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(0)}</span>
                </span>
                <span title="Cursor Y">
                    "Y"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(1)}</span>
                </span>
                <span title="Cursor Z">
                    "Z"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(2)}</span>
                </span>
            </div>
            <span class="mx-1 h-5 w-px bg-white/10"></span>
            <div
                class="flex items-center gap-2 px-1 font-mono text-code-md tabular-nums text-on-surface-variant"
                title="Placed slots on map / current selection"
            >
                <span>
                    "OBJ"
                    <span class="ml-1 text-on-surface">{move || obj_count.get()}</span>
                </span>
                <span>
                    "SEL"
                    <span class="ml-1 text-on-surface">{move || sel_count.get()}</span>
                </span>
                <span title="Estimated save payload">
                    "SZ"
                    <span class="ml-1 text-on-surface">
                        {move || {
                            sz_bytes
                                .and_then(|s| s.get())
                                .map_or_else(
                                    || "—".to_string(),
                                    crate::mission_size::format_bytes,
                                )
                        }}
                    </span>
                </span>
            </div>
        </div>
    }
}
